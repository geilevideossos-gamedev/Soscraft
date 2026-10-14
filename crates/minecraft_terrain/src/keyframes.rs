use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result, anyhow, bail};
use glam::Vec3;

use crate::class_file::{self, ClassFile, Constant, Member};

pub const DEFINITIONS: &str = "net/minecraft/client/animation/definitions/";
pub const DEGREES: f32 = 0.017_453_292;

const ANIMATION: &str = "net/minecraft/client/animation/";
const DEFINITION: &str = "Lnet/minecraft/client/animation/AnimationDefinition;";
const STOPPED: i32 = i32::MIN;
const ACONST_NULL: u8 = 0x01;
const FCONST_0: u8 = 0x0b;
const FCONST_2: u8 = 0x0d;
const DCONST_0: u8 = 0x0e;
const DCONST_1: u8 = 0x0f;
const AASTORE: u8 = 0x53;
const POP: u8 = 0x57;
const DUP: u8 = 0x59;
const RETURN: u8 = 0xb1;
const ANEWARRAY: u8 = 0xbd;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Target {
    Position,
    Rotation,
    Scale,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Interpolation {
    Linear,
    CatmullRom,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keyframe {
    pub timestamp: f32,
    pub pre: Vec3,
    pub post: Vec3,
    pub interpolation: Interpolation,
}

impl Keyframe {
    pub fn new(timestamp: f32, target: Vec3, interpolation: Interpolation) -> Keyframe {
        Keyframe {
            timestamp,
            pre: target,
            post: target,
            interpolation,
        }
    }
}

pub fn pos_vec(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, -y, z)
}

pub fn degree_vec(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x * DEGREES, y * DEGREES, z * DEGREES)
}

pub fn scale_vec(x: f64, y: f64, z: f64) -> Vec3 {
    Vec3::new((x - 1.0) as f32, (y - 1.0) as f32, (z - 1.0) as f32)
}

pub fn catmullrom(alpha: f32, p0: f32, p1: f32, p2: f32, p3: f32) -> f32 {
    0.5 * (2.0 * p1
        + (p2 - p0) * alpha
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * alpha * alpha
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * alpha * alpha * alpha)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Channel {
    pub target: Target,
    pub keyframes: Vec<Keyframe>,
}

impl Channel {
    pub fn new(target: Target, keyframes: Vec<Keyframe>) -> Channel {
        Channel { target, keyframes }
    }

    pub fn sample(&self, seconds: f32, scale: f32) -> Vec3 {
        let keys = &self.keyframes;
        if keys.is_empty() {
            return Vec3::ZERO;
        }
        let (mut from, mut len) = (0usize, keys.len());
        while len > 0 {
            let half = len / 2;
            if seconds <= keys[from + half].timestamp {
                len = half;
            } else {
                from += half + 1;
                len -= half + 1;
            }
        }
        let prev = from.saturating_sub(1);
        let next = (prev + 1).min(keys.len() - 1);
        let alpha = if next != prev {
            ((seconds - keys[prev].timestamp) / (keys[next].timestamp - keys[prev].timestamp))
                .clamp(0.0, 1.0)
        } else {
            0.0
        };
        match keys[next].interpolation {
            Interpolation::Linear => {
                let (a, b) = (keys[prev].post, keys[next].pre);
                ((b - a) * alpha + a) * scale
            }
            Interpolation::CatmullRom => {
                let p0 = keys[prev.saturating_sub(1)].post;
                let p1 = keys[prev].post;
                let p2 = keys[next].post;
                let p3 = keys[(next + 1).min(keys.len() - 1)].post;
                Vec3::new(
                    catmullrom(alpha, p0.x, p1.x, p2.x, p3.x) * scale,
                    catmullrom(alpha, p0.y, p1.y, p2.y, p3.y) * scale,
                    catmullrom(alpha, p0.z, p1.z, p2.z, p3.z) * scale,
                )
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Offsets {
    pub position: Vec3,
    pub rotation: Vec3,
    pub scale: Vec3,
}

impl Offsets {
    pub fn add(&mut self, target: Target, value: Vec3) {
        match target {
            Target::Position => self.position += value,
            Target::Rotation => self.rotation += value,
            Target::Scale => self.scale += value,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Moved {
    parts: Vec<(String, Offsets)>,
}

impl Moved {
    pub fn add(&mut self, bone: &str, target: Target, value: Vec3) {
        match self.parts.iter_mut().find(|(name, _)| name == bone) {
            Some((_, offsets)) => offsets.add(target, value),
            None => {
                let mut offsets = Offsets::default();
                offsets.add(target, value);
                self.parts.push((bone.to_owned(), offsets));
            }
        }
    }

    pub fn get(&self, bone: &str) -> Offsets {
        self.parts
            .iter()
            .find(|(name, _)| name == bone)
            .map_or(Offsets::default(), |(_, offsets)| *offsets)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, Offsets)> {
        self.parts
            .iter()
            .map(|(name, offsets)| (name.as_str(), *offsets))
    }

    pub fn clear(&mut self) {
        self.parts.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimationState {
    start_tick: i32,
}

impl Default for AnimationState {
    fn default() -> AnimationState {
        AnimationState::STOPPED
    }
}

impl AnimationState {
    pub const STOPPED: AnimationState = AnimationState {
        start_tick: STOPPED,
    };

    pub fn started(tick: i32) -> AnimationState {
        AnimationState { start_tick: tick }
    }

    pub fn from_start(start: Option<i32>) -> AnimationState {
        start.map_or(AnimationState::STOPPED, AnimationState::started)
    }

    pub fn start(&mut self, tick: i32) {
        self.start_tick = tick;
    }

    pub fn start_if_stopped(&mut self, tick: i32) {
        if !self.is_started() {
            self.start(tick);
        }
    }

    pub fn animate_when(&mut self, condition: bool, tick: i32) {
        if condition {
            self.start_if_stopped(tick);
        } else {
            self.stop();
        }
    }

    pub fn stop(&mut self) {
        self.start_tick = STOPPED;
    }

    pub fn is_started(&self) -> bool {
        self.start_tick != STOPPED
    }

    pub fn start_tick(&self) -> Option<i32> {
        self.is_started().then_some(self.start_tick)
    }

    pub fn fast_forward(&mut self, ticks: i32, scale: f32) {
        if self.is_started() {
            self.start_tick -= (ticks as f32 * scale) as i32;
        }
    }

    pub fn time_in_millis(&self, age_in_ticks: f32) -> i64 {
        ((age_in_ticks - self.start_tick as f32) * 50.0) as i64
    }
}

pub fn age_in_ticks(tick_count: i32, partial: f32) -> f32 {
    tick_count as f32 + partial
}

#[derive(Clone, Debug, PartialEq)]
pub struct Definition {
    pub length: f32,
    pub looping: bool,
    pub bones: Vec<(String, Vec<Channel>)>,
}

impl Definition {
    pub fn new(length: f32, looping: bool) -> Definition {
        Definition {
            length,
            looping,
            bones: Vec::new(),
        }
    }

    pub fn add(&mut self, bone: &str, channel: Channel) {
        match self.bones.iter_mut().find(|(name, _)| name == bone) {
            Some((_, channels)) => channels.push(channel),
            None => self.bones.push((bone.to_owned(), vec![channel])),
        }
    }

    pub fn channel(&self, bone: &str, target: Target) -> Option<&Channel> {
        self.bones
            .iter()
            .filter(|(name, _)| name == bone)
            .flat_map(|(_, channels)| channels)
            .find(|channel| channel.target == target)
    }

    pub fn keyframe_count(&self) -> usize {
        self.bones
            .iter()
            .flat_map(|(_, channels)| channels)
            .map(|channel| channel.keyframes.len())
            .sum()
    }

    pub fn channel_count(&self) -> usize {
        self.bones.iter().map(|(_, channels)| channels.len()).sum()
    }

    pub fn elapsed_seconds(&self, millis: i64) -> f32 {
        let seconds = millis as f32 / 1000.0;
        if self.looping {
            seconds % self.length
        } else {
            seconds
        }
    }

    pub fn apply(&self, millis: i64, scale: f32, moved: &mut Moved) {
        let seconds = self.elapsed_seconds(millis);
        for (bone, channels) in &self.bones {
            for channel in channels {
                moved.add(bone, channel.target, channel.sample(seconds, scale));
            }
        }
    }

    pub fn apply_state(
        &self,
        state: AnimationState,
        age_in_ticks: f32,
        speed: f32,
        moved: &mut Moved,
    ) {
        if state.is_started() {
            let millis = (state.time_in_millis(age_in_ticks) as f32 * speed) as i64;
            self.apply(millis, 1.0, moved);
        }
    }

    pub fn apply_walk(
        &self,
        walk_position: f32,
        walk_speed: f32,
        speed_multiplier: f32,
        amplitude: f32,
        moved: &mut Moved,
    ) {
        let millis = (walk_position * 50.0 * speed_multiplier) as i64;
        self.apply(millis, (walk_speed * amplitude).min(1.0), moved);
    }

    pub fn apply_static(&self, moved: &mut Moved) {
        self.apply(0, 1.0, moved);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnimationClass {
    pub name: String,
    pub definitions: Vec<(String, Arc<Definition>)>,
}

impl AnimationClass {
    pub fn get(&self, field: &str) -> Option<&Arc<Definition>> {
        self.definitions
            .iter()
            .find(|(name, _)| name == field)
            .map(|(_, definition)| definition)
    }
}

#[derive(Clone, Debug)]
enum Value {
    Int(i32),
    Float(f32),
    Double(f64),
    Text(String),
    Target(Target),
    Interpolation(Interpolation),
    Vector(Vec3),
    Object(usize),
}

#[derive(Clone, Debug)]
enum Object {
    New(String),
    Keyframe(Keyframe),
    Array(Vec<Option<Keyframe>>),
    Channel(Channel),
    Builder(Definition),
    Definition(Definition),
}

struct Machine<'a> {
    class: &'a ClassFile,
    stack: Vec<Value>,
    heap: Vec<Object>,
}

impl Machine<'_> {
    fn pop(&mut self) -> Result<Value> {
        self.stack
            .pop()
            .ok_or_else(|| anyhow!("the stack is empty"))
    }

    fn float(&mut self) -> Result<f32> {
        match self.pop()? {
            Value::Float(value) => Ok(value),
            other => bail!("expected a float, found {other:?}"),
        }
    }

    fn double(&mut self) -> Result<f64> {
        match self.pop()? {
            Value::Double(value) => Ok(value),
            other => bail!("expected a double, found {other:?}"),
        }
    }

    fn int(&mut self) -> Result<i32> {
        match self.pop()? {
            Value::Int(value) => Ok(value),
            other => bail!("expected an int, found {other:?}"),
        }
    }

    fn vector(&mut self) -> Result<Vec3> {
        match self.pop()? {
            Value::Vector(value) => Ok(value),
            other => bail!("expected a vector, found {other:?}"),
        }
    }

    fn reference(&mut self) -> Result<usize> {
        match self.pop()? {
            Value::Object(at) => Ok(at),
            other => bail!("expected an object, found {other:?}"),
        }
    }

    fn alloc(&mut self, object: Object) -> Value {
        self.heap.push(object);
        Value::Object(self.heap.len() - 1)
    }

    fn take(&mut self, at: usize) -> Result<Object> {
        let slot = self
            .heap
            .get_mut(at)
            .ok_or_else(|| anyhow!("no object {at}"))?;
        Ok(std::mem::replace(slot, Object::New(String::new())))
    }

    fn builder(&mut self) -> Result<usize> {
        let at = self.reference()?;
        match self.heap.get(at) {
            Some(Object::Builder(_)) => Ok(at),
            other => bail!("expected a definition builder, found {other:?}"),
        }
    }

    fn construct(&mut self, at: usize, class: &str, object: Object) -> Result<()> {
        match self.heap.get(at) {
            Some(Object::New(name)) if name == class => {
                self.heap[at] = object;
                Ok(())
            }
            other => bail!("{class}.<init> on {other:?}"),
        }
    }

    fn constant(&mut self, index: u16) -> Result<()> {
        let value = match self.class.constant(index) {
            Some(Constant::Float(value)) => Value::Float(*value),
            Some(Constant::Double(value)) => Value::Double(*value),
            Some(Constant::Integer(value)) => Value::Int(*value),
            Some(Constant::String(_)) => Value::Text(
                self.class
                    .string(index)
                    .ok_or_else(|| anyhow!("string constant {index} has no text"))?
                    .to_owned(),
            ),
            other => bail!("unsupported constant {other:?}"),
        };
        self.stack.push(value);
        Ok(())
    }

    fn member(&self, index: u16) -> Result<(String, Member)> {
        let member = self
            .class
            .member(index)
            .ok_or_else(|| anyhow!("constant {index} is not a member"))?;
        let local = member
            .class
            .strip_prefix(ANIMATION)
            .unwrap_or(&member.class)
            .to_owned();
        Ok((local, member))
    }

    fn get_static(&mut self, index: u16) -> Result<()> {
        let (owner, member) = self.member(index)?;
        let value = match (owner.as_str(), member.name.as_str()) {
            ("AnimationChannel$Targets", "POSITION") => Value::Target(Target::Position),
            ("AnimationChannel$Targets", "ROTATION") => Value::Target(Target::Rotation),
            ("AnimationChannel$Targets", "SCALE") => Value::Target(Target::Scale),
            ("AnimationChannel$Interpolations", "LINEAR") => {
                Value::Interpolation(Interpolation::Linear)
            }
            ("AnimationChannel$Interpolations", "CATMULLROM") => {
                Value::Interpolation(Interpolation::CatmullRom)
            }
            _ => bail!("unknown field {}.{}", member.class, member.name),
        };
        self.stack.push(value);
        Ok(())
    }

    fn invoke_static(&mut self, index: u16) -> Result<()> {
        let (owner, member) = self.member(index)?;
        let value = match (owner.as_str(), member.name.as_str()) {
            ("KeyframeAnimations", "posVec" | "degreeVec") => {
                let (z, y, x) = (self.float()?, self.float()?, self.float()?);
                Value::Vector(if member.name == "posVec" {
                    pos_vec(x, y, z)
                } else {
                    degree_vec(x, y, z)
                })
            }
            ("KeyframeAnimations", "scaleVec") => {
                let (z, y, x) = (self.double()?, self.double()?, self.double()?);
                Value::Vector(scale_vec(x, y, z))
            }
            ("AnimationDefinition$Builder", "withLength") => {
                let length = self.float()?;
                self.alloc(Object::Builder(Definition::new(length, false)))
            }
            _ => bail!("unknown method {}.{}", member.class, member.name),
        };
        self.stack.push(value);
        Ok(())
    }

    fn invoke_special(&mut self, index: u16) -> Result<()> {
        let (owner, member) = self.member(index)?;
        if member.name != "<init>" {
            bail!("unknown method {}.{}", member.class, member.name);
        }
        let arguments = class_file::parameters(&member.descriptor).len();
        match (owner.as_str(), arguments) {
            ("Keyframe", 3 | 4) => {
                let interpolation = match self.pop()? {
                    Value::Interpolation(value) => value,
                    other => bail!("expected an interpolation, found {other:?}"),
                };
                let post = self.vector()?;
                let pre = if arguments == 4 { self.vector()? } else { post };
                let timestamp = self.float()?;
                let at = self.reference()?;
                let keyframe = Keyframe {
                    timestamp,
                    pre,
                    post,
                    interpolation,
                };
                self.construct(at, &member.class, Object::Keyframe(keyframe))
            }
            ("AnimationChannel", 2) => {
                let array = self.reference()?;
                let keyframes = match self.take(array)? {
                    Object::Array(slots) => slots
                        .into_iter()
                        .collect::<Option<Vec<Keyframe>>>()
                        .ok_or_else(|| anyhow!("a channel has an empty keyframe slot"))?,
                    other => bail!("expected a keyframe array, found {other:?}"),
                };
                let target = match self.pop()? {
                    Value::Target(value) => value,
                    other => bail!("expected a target, found {other:?}"),
                };
                let at = self.reference()?;
                self.construct(
                    at,
                    &member.class,
                    Object::Channel(Channel::new(target, keyframes)),
                )
            }
            _ => bail!("unknown constructor {}{}", member.class, member.descriptor),
        }
    }

    fn invoke_virtual(&mut self, index: u16) -> Result<()> {
        let (owner, member) = self.member(index)?;
        if owner != "AnimationDefinition$Builder" {
            bail!("unknown method {}.{}", member.class, member.name);
        }
        match member.name.as_str() {
            "looping" => {
                let at = self.builder()?;
                if let Object::Builder(definition) = &mut self.heap[at] {
                    definition.looping = true;
                }
                self.stack.push(Value::Object(at));
            }
            "addAnimation" => {
                let channel = match self.reference().and_then(|at| self.take(at))? {
                    Object::Channel(channel) => channel,
                    other => bail!("expected a channel, found {other:?}"),
                };
                let bone = match self.pop()? {
                    Value::Text(bone) => bone,
                    other => bail!("expected a bone name, found {other:?}"),
                };
                let at = self.builder()?;
                if let Object::Builder(definition) = &mut self.heap[at] {
                    definition.add(&bone, channel);
                }
                self.stack.push(Value::Object(at));
            }
            "build" => {
                let at = self.builder()?;
                let Object::Builder(definition) = self.take(at)? else {
                    bail!("the builder vanished");
                };
                let built = self.alloc(Object::Definition(definition));
                self.stack.push(built);
            }
            _ => bail!("unknown method {}.{}", member.class, member.name),
        }
        Ok(())
    }

    fn put_static(&mut self, index: u16, out: &mut Vec<(String, Arc<Definition>)>) -> Result<()> {
        let member = self
            .class
            .member(index)
            .ok_or_else(|| anyhow!("constant {index} is not a field"))?;
        if member.class != self.class.name || member.descriptor != DEFINITION {
            bail!("unknown static {}.{}", member.class, member.name);
        }
        match self.reference().and_then(|at| self.take(at))? {
            Object::Definition(definition) => {
                out.push((member.name, Arc::new(definition)));
                Ok(())
            }
            other => bail!("{} is not a built definition: {other:?}", member.name),
        }
    }

    fn array_store(&mut self) -> Result<()> {
        let value = self.reference()?;
        let index = self.int()?;
        let array = self.reference()?;
        let keyframe = match self.heap.get(value) {
            Some(Object::Keyframe(keyframe)) => *keyframe,
            other => bail!("expected a keyframe, found {other:?}"),
        };
        match self.heap.get_mut(array) {
            Some(Object::Array(slots)) => {
                let slot = usize::try_from(index)
                    .ok()
                    .and_then(|index| slots.get_mut(index))
                    .ok_or_else(|| anyhow!("keyframe index {index} is out of bounds"))?;
                *slot = Some(keyframe);
                Ok(())
            }
            other => bail!("expected a keyframe array, found {other:?}"),
        }
    }
}

pub fn read_class(class: &ClassFile) -> Result<AnimationClass> {
    let code = class
        .method("<clinit>")
        .and_then(|method| method.code.as_deref())
        .ok_or_else(|| anyhow!("{} has no static initialiser", class.name))?;
    let mut machine = Machine {
        class,
        stack: Vec::new(),
        heap: Vec::new(),
    };
    let mut definitions = Vec::new();
    for instruction in class_file::instructions(code)? {
        let at = instruction.at;
        let index = instruction.index;
        let step = match instruction.op {
            class_file::ICONST_M1..=class_file::ICONST_5
            | class_file::BIPUSH
            | class_file::SIPUSH => {
                machine.stack.push(Value::Int(instruction.value));
                Ok(())
            }
            FCONST_0..=FCONST_2 => {
                machine
                    .stack
                    .push(Value::Float(f32::from(instruction.op - FCONST_0)));
                Ok(())
            }
            DCONST_0 | DCONST_1 => {
                machine
                    .stack
                    .push(Value::Double(f64::from(instruction.op - DCONST_0)));
                Ok(())
            }
            class_file::LDC | class_file::LDC_W | class_file::LDC2_W => machine.constant(index),
            DUP => match machine.stack.last().cloned() {
                Some(top) => {
                    machine.stack.push(top);
                    Ok(())
                }
                None => Err(anyhow!("dup on an empty stack")),
            },
            POP => machine.pop().map(|_| ()),
            class_file::NEW => {
                let name = class
                    .class_name(index)
                    .ok_or_else(|| anyhow!("constant {index} is not a class"))?
                    .to_owned();
                let object = machine.alloc(Object::New(name));
                machine.stack.push(object);
                Ok(())
            }
            ANEWARRAY => machine.int().and_then(|count| {
                let count = usize::try_from(count).context("a negative array size")?;
                let array = machine.alloc(Object::Array(vec![None; count]));
                machine.stack.push(array);
                Ok(())
            }),
            AASTORE => machine.array_store(),
            class_file::GETSTATIC => machine.get_static(index),
            class_file::INVOKESTATIC => machine.invoke_static(index),
            class_file::INVOKESPECIAL => machine.invoke_special(index),
            class_file::INVOKEVIRTUAL => machine.invoke_virtual(index),
            class_file::PUTSTATIC => machine.put_static(index, &mut definitions),
            RETURN => break,
            ACONST_NULL => Err(anyhow!("a null keyframe or channel")),
            op => Err(anyhow!("unsupported opcode {op:#04x}")),
        };
        step.with_context(|| format!("{} <clinit> at {at}", class.name))?;
    }
    if definitions.is_empty() {
        bail!("{} defines no animations", class.name);
    }
    Ok(AnimationClass {
        name: class.name.clone(),
        definitions,
    })
}

pub fn class_path(name: &str) -> String {
    if name.contains('/') {
        name.to_owned()
    } else {
        format!("{DEFINITIONS}{name}")
    }
}

pub fn read_from_jar(jar: &Path, name: &str) -> Result<AnimationClass> {
    let path = class_path(name);
    let file =
        std::fs::File::open(jar).with_context(|| format!("cannot open {}", jar.display()))?;
    let mut archive = zip::ZipArchive::new(file)?;
    let mut entry = archive
        .by_name(&format!("{path}.class"))
        .with_context(|| format!("no {path}.class in the jar"))?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    read_class(&ClassFile::parse(&bytes)?)
}

pub struct AnimationLibrary {
    jar: Option<PathBuf>,
    classes: Mutex<HashMap<String, Result<Arc<AnimationClass>, String>>>,
}

impl AnimationLibrary {
    pub fn new(jar: Option<PathBuf>) -> AnimationLibrary {
        AnimationLibrary {
            jar,
            classes: Mutex::new(HashMap::new()),
        }
    }

    pub fn class(&self, name: &str) -> Result<Arc<AnimationClass>> {
        let path = class_path(name);
        let mut classes = self
            .classes
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let read = classes.entry(path).or_insert_with_key(|path| {
            let jar = self
                .jar
                .as_deref()
                .ok_or_else(|| "no client jar for keyframe animations".to_owned())?;
            read_from_jar(jar, path)
                .map(Arc::new)
                .map_err(|error| format!("{error:#}"))
        });
        read.clone().map_err(|error| anyhow!(error))
    }

    pub fn definition(&self, class: &str, field: &str) -> Option<Arc<Definition>> {
        self.class(class).ok()?.get(field).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class_file::Method;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    fn channel(interpolation: Interpolation, keys: &[(f32, f32)]) -> Channel {
        Channel::new(
            Target::Position,
            keys.iter()
                .map(|&(t, x)| Keyframe::new(t, Vec3::new(x, 0.0, 0.0), interpolation))
                .collect(),
        )
    }

    #[test]
    fn linear_keyframes_interpolate_between_their_pair() {
        let keys = channel(
            Interpolation::Linear,
            &[(0.0, 0.0), (0.125, 20.0), (0.5, 0.0)],
        );
        assert_eq!(keys.sample(0.0, 1.0).x, 0.0);
        assert_eq!(keys.sample(0.0625, 1.0).x, 10.0);
        assert_eq!(keys.sample(0.125, 1.0).x, 20.0);
        assert_eq!(keys.sample(0.3125, 1.0).x, 10.0);
        assert_eq!(keys.sample(0.3125, 0.5).x, 5.0);
        assert_eq!(keys.sample(-1.0, 1.0).x, 0.0);
        assert_eq!(keys.sample(2.0, 1.0).x, 0.0);
        let lone = channel(Interpolation::Linear, &[(0.0, 180.0)]);
        assert_eq!(lone.sample(0.3, 1.0).x, 180.0);
        assert_eq!(
            channel(Interpolation::Linear, &[]).sample(0.3, 1.0),
            Vec3::ZERO
        );
    }

    #[test]
    fn linear_keyframes_go_from_post_to_pre() {
        let keys = Channel::new(
            Target::Rotation,
            vec![
                Keyframe {
                    timestamp: 0.0,
                    pre: Vec3::splat(100.0),
                    post: Vec3::splat(2.0),
                    interpolation: Interpolation::Linear,
                },
                Keyframe {
                    timestamp: 1.0,
                    pre: Vec3::splat(6.0),
                    post: Vec3::splat(-50.0),
                    interpolation: Interpolation::Linear,
                },
            ],
        );
        assert_eq!(keys.sample(0.5, 1.0), Vec3::splat(4.0));
    }

    #[test]
    fn catmullrom_keyframes_follow_the_spline_through_their_neighbours() {
        assert_eq!(catmullrom(0.0, 9.0, 1.0, 5.0, 7.0), 1.0);
        assert_eq!(catmullrom(1.0, 9.0, 1.0, 5.0, 7.0), 5.0);
        assert_eq!(catmullrom(0.5, 0.0, 0.0, 10.0, 10.0), 5.0);
        assert_eq!(catmullrom(0.5, 0.0, 10.0, 20.0, 30.0), 15.0);
        let keys = channel(
            Interpolation::CatmullRom,
            &[(0.0, 0.0), (1.0, 10.0), (2.0, 0.0), (3.0, 10.0)],
        );
        assert_eq!(keys.sample(1.0, 1.0).x, 10.0);
        assert_eq!(keys.sample(1.5, 1.0).x, 5.0);
        assert!(close(keys.sample(0.5, 1.0).x, 5.625));
        assert!(close(keys.sample(2.5, 1.0).x, 4.375));
        assert!(close(keys.sample(1.25, 1.0).x, 8.4375));
        assert!(close(keys.sample(1.25, 2.0).x, 16.875));
        assert_eq!(keys.sample(9.0, 1.0).x, 10.0);
    }

    #[test]
    fn the_next_keyframe_picks_the_interpolation() {
        let keys = Channel::new(
            Target::Position,
            vec![
                Keyframe::new(0.0, Vec3::ZERO, Interpolation::CatmullRom),
                Keyframe::new(1.0, Vec3::splat(10.0), Interpolation::Linear),
                Keyframe::new(2.0, Vec3::ZERO, Interpolation::CatmullRom),
                Keyframe::new(3.0, Vec3::splat(10.0), Interpolation::Linear),
            ],
        );
        assert_eq!(keys.sample(0.25, 1.0).x, 2.5);
        assert!(close(keys.sample(1.25, 1.0).x, 8.4375));
    }

    #[test]
    fn vectors_are_built_as_keyframe_animations_builds_them() {
        assert_eq!(pos_vec(1.0, 2.0, 3.0), Vec3::new(1.0, -2.0, 3.0));
        assert_eq!(degree_vec(180.0, 0.0, -90.0).x, 180.0 * 0.017_453_292_f32);
        assert_eq!(DEGREES, std::f32::consts::PI / 180.0);
        assert_eq!(scale_vec(1.5, 1.0, 0.5), Vec3::new(0.5, 0.0, -0.5));
    }

    fn spin() -> Definition {
        let mut definition = Definition::new(0.5, true);
        definition.add(
            "head",
            Channel::new(
                Target::Rotation,
                vec![
                    Keyframe::new(0.0, Vec3::ZERO, Interpolation::Linear),
                    Keyframe::new(0.5, Vec3::new(1.0, 0.0, 0.0), Interpolation::Linear),
                ],
            ),
        );
        definition.add(
            "head",
            Channel::new(
                Target::Position,
                vec![Keyframe::new(
                    0.0,
                    Vec3::new(0.0, -2.0, 0.0),
                    Interpolation::Linear,
                )],
            ),
        );
        definition.add(
            "body",
            Channel::new(
                Target::Scale,
                vec![Keyframe::new(0.0, Vec3::splat(0.5), Interpolation::Linear)],
            ),
        );
        definition
    }

    #[test]
    fn a_looping_animation_wraps_and_a_finished_one_holds() {
        let mut definition = spin();
        assert_eq!(definition.elapsed_seconds(250), 0.25);
        assert_eq!(definition.elapsed_seconds(750), 0.25);
        let mut moved = Moved::default();
        definition.apply(750, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.5);
        assert_eq!(moved.get("head").position.y, -2.0);
        assert_eq!(moved.get("body").scale, Vec3::splat(0.5));
        assert_eq!(moved.get("tail"), Offsets::default());
        definition.looping = false;
        moved.clear();
        definition.apply(750, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 1.0);
        moved.clear();
        definition.apply_static(&mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.0);
        assert_eq!(definition.channel_count(), 3);
        assert_eq!(definition.keyframe_count(), 4);
        assert_eq!(
            definition
                .channel("head", Target::Position)
                .map(|c| c.keyframes.len()),
            Some(1)
        );
        assert!(definition.channel("body", Target::Rotation).is_none());
    }

    #[test]
    fn offsets_add_up_over_animations() {
        let definition = spin();
        let mut moved = Moved::default();
        definition.apply(250, 1.0, &mut moved);
        definition.apply(250, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 1.0);
        assert_eq!(moved.get("head").position.y, -4.0);
        assert_eq!(moved.iter().count(), 2);
    }

    #[test]
    fn an_animation_state_runs_from_its_start_tick_with_the_partial() {
        let mut state = AnimationState::default();
        assert!(!state.is_started());
        assert_eq!(state.start_tick(), None);
        let definition = spin();
        let mut moved = Moved::default();
        definition.apply_state(state, 10.0, 1.0, &mut moved);
        assert_eq!(moved, Moved::default());
        state.start(100);
        state.start_if_stopped(120);
        assert_eq!(state.start_tick(), Some(100));
        let age = age_in_ticks(102, 0.5);
        assert_eq!(state.time_in_millis(age), 125);
        definition.apply_state(state, age, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.25);
        moved.clear();
        definition.apply_state(state, age, 2.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.5);
        assert_eq!(state.time_in_millis(age_in_ticks(110, 0.0)), 500);
        state.fast_forward(4, 0.5);
        assert_eq!(state.start_tick(), Some(98));
        state.animate_when(false, 130);
        assert!(!state.is_started());
        state.fast_forward(4, 1.0);
        assert!(!state.is_started());
        state.animate_when(true, 130);
        state.animate_when(true, 140);
        assert_eq!(state.start_tick(), Some(130));
        assert_eq!(
            AnimationState::from_start(Some(7)),
            AnimationState::started(7)
        );
        assert_eq!(AnimationState::from_start(None), AnimationState::STOPPED);
        state.stop();
        assert_eq!(state, AnimationState::STOPPED);
    }

    #[test]
    fn a_walk_animation_runs_on_the_walk_position_scaled_by_its_speed() {
        let definition = spin();
        let mut moved = Moved::default();
        definition.apply_walk(5.0, 0.5, 1.0, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.25);
        moved.clear();
        definition.apply_walk(5.0, 3.0, 1.0, 1.0, &mut moved);
        assert_eq!(moved.get("head").rotation.x, 0.5);
    }

    #[derive(Default)]
    struct Pool(Vec<Constant>);

    impl Pool {
        fn add(&mut self, constant: Constant) -> u16 {
            let wide = matches!(constant, Constant::Double(_));
            self.0.push(constant);
            if wide {
                self.0.push(Constant::Unused);
            }
            (self.0.len() - if wide { 2 } else { 1 }) as u16
        }

        fn utf8(&mut self, text: &str) -> u16 {
            self.add(Constant::Utf8(text.to_owned()))
        }

        fn class(&mut self, name: &str) -> u16 {
            let name = self.utf8(name);
            self.add(Constant::Class(name))
        }

        fn member(&mut self, field: bool, class: &str, name: &str, descriptor: &str) -> u16 {
            let class = self.class(class);
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            let nat = self.add(Constant::NameAndType(name, descriptor));
            self.add(if field {
                Constant::Field(class, nat)
            } else {
                Constant::Method(class, nat)
            })
        }
    }

    fn synthetic(extra: &[u8]) -> ClassFile {
        let mut pool = Pool(vec![Constant::Unused]);
        let this = "test/SpinAnimation";
        let animation = |name: &str| format!("{ANIMATION}{name}");
        let builder = animation("AnimationDefinition$Builder");
        let returns_builder = format!("L{builder};");
        let with_length = pool.member(
            false,
            &builder,
            "withLength",
            &format!("(F){returns_builder}"),
        );
        let looping = pool.member(false, &builder, "looping", &format!("(){returns_builder}"));
        let add = pool.member(
            false,
            &builder,
            "addAnimation",
            &format!("(Ljava/lang/String;L{ANIMATION}AnimationChannel;){returns_builder}"),
        );
        let build = pool.member(false, &builder, "build", &format!("(){DEFINITION}"));
        let head_text = pool.utf8("head");
        let head = pool.add(Constant::String(head_text));
        let channel_class = pool.class(&animation("AnimationChannel"));
        let keyframe_class = pool.class(&animation("Keyframe"));
        let rotation = pool.member(
            true,
            &animation("AnimationChannel$Targets"),
            "ROTATION",
            "Lnet/minecraft/client/animation/AnimationChannel$Target;",
        );
        let scale = pool.member(
            true,
            &animation("AnimationChannel$Targets"),
            "SCALE",
            "Lnet/minecraft/client/animation/AnimationChannel$Target;",
        );
        let linear = pool.member(
            true,
            &animation("AnimationChannel$Interpolations"),
            "LINEAR",
            "Lnet/minecraft/client/animation/AnimationChannel$Interpolation;",
        );
        let catmullrom = pool.member(
            true,
            &animation("AnimationChannel$Interpolations"),
            "CATMULLROM",
            "Lnet/minecraft/client/animation/AnimationChannel$Interpolation;",
        );
        let degree = pool.member(
            false,
            &animation("KeyframeAnimations"),
            "degreeVec",
            "(FFF)Lorg/joml/Vector3f;",
        );
        let scale_vec = pool.member(
            false,
            &animation("KeyframeAnimations"),
            "scaleVec",
            "(DDD)Lorg/joml/Vector3f;",
        );
        let keyframe_init = pool.member(
            false,
            &animation("Keyframe"),
            "<init>",
            "(FLorg/joml/Vector3fc;Lnet/minecraft/client/animation/AnimationChannel$Interpolation;)V",
        );
        let channel_init = pool.member(
            false,
            &animation("AnimationChannel"),
            "<init>",
            "(Lnet/minecraft/client/animation/AnimationChannel$Target;[Lnet/minecraft/client/animation/Keyframe;)V",
        );
        let ninety = pool.add(Constant::Float(90.0));
        let half = pool.add(Constant::Float(0.5));
        let two = pool.add(Constant::Double(2.0));
        let field = pool.member(true, this, "SPIN", DEFINITION);
        let this_class = pool.class(this);
        let wide = |op: u8, index: u16| {
            let [high, low] = index.to_be_bytes();
            vec![op, high, low]
        };
        let mut code = Vec::new();
        code.extend(wide(class_file::LDC_W, half));
        code.extend(wide(class_file::INVOKESTATIC, with_length));
        code.extend(wide(class_file::INVOKEVIRTUAL, looping));
        for (target, frames) in [(rotation, 2u8), (scale, 1)] {
            code.extend(wide(class_file::LDC_W, head));
            code.extend(wide(class_file::NEW, channel_class));
            code.push(DUP);
            code.extend(wide(class_file::GETSTATIC, target));
            code.extend([class_file::BIPUSH, frames]);
            code.extend(wide(ANEWARRAY, keyframe_class));
            for slot in 0..frames {
                code.push(DUP);
                code.push(0x03 + slot);
                code.extend(wide(class_file::NEW, keyframe_class));
                code.push(DUP);
                if target == scale {
                    code.push(FCONST_0);
                    code.extend(wide(class_file::LDC2_W, two));
                    code.extend([DCONST_1, DCONST_0]);
                    code.extend(wide(class_file::INVOKESTATIC, scale_vec));
                    code.extend(wide(class_file::GETSTATIC, linear));
                } else {
                    code.push(FCONST_0 + slot);
                    code.extend([FCONST_0, FCONST_0]);
                    code.extend(wide(class_file::LDC_W, ninety));
                    code.extend(wide(class_file::INVOKESTATIC, degree));
                    code.extend(wide(class_file::GETSTATIC, catmullrom));
                }
                code.extend(wide(class_file::INVOKESPECIAL, keyframe_init));
                code.push(AASTORE);
            }
            code.extend(wide(class_file::INVOKESPECIAL, channel_init));
            code.extend(wide(class_file::INVOKEVIRTUAL, add));
        }
        code.extend(wide(class_file::INVOKEVIRTUAL, build));
        code.extend(wide(class_file::PUTSTATIC, field));
        code.extend(extra);
        code.push(RETURN);
        let mut class = ClassFile {
            name: String::new(),
            pool: pool.0,
            fields: Vec::new(),
            methods: vec![Method {
                access: 8,
                name: "<clinit>".to_owned(),
                descriptor: "()V".to_owned(),
                max_locals: 0,
                code: Some(code),
            }],
            bootstraps: Vec::new(),
        };
        class.name = class.class_name(this_class).unwrap().to_owned();
        class
    }

    #[test]
    fn the_reader_runs_a_static_initialiser_of_builder_calls() {
        let read = read_class(&synthetic(&[])).unwrap();
        assert_eq!(read.name, "test/SpinAnimation");
        let spin = read.get("SPIN").unwrap();
        assert_eq!(spin.length, 0.5);
        assert!(spin.looping);
        assert_eq!(spin.bones.len(), 1);
        let rotation = spin.channel("head", Target::Rotation).unwrap();
        assert_eq!(
            rotation.keyframes,
            vec![
                Keyframe::new(0.0, degree_vec(0.0, 0.0, 90.0), Interpolation::CatmullRom),
                Keyframe::new(1.0, degree_vec(0.0, 0.0, 90.0), Interpolation::CatmullRom),
            ]
        );
        let scale = spin.channel("head", Target::Scale).unwrap();
        assert_eq!(
            scale.keyframes,
            vec![Keyframe::new(
                0.0,
                Vec3::new(1.0, 0.0, -1.0),
                Interpolation::Linear
            )]
        );
        assert!(read.get("WALK").is_none());
    }

    #[test]
    fn the_reader_refuses_bytecode_it_does_not_know() {
        let error = read_class(&synthetic(&[0x60])).unwrap_err();
        assert!(format!("{error:#}").contains("unsupported opcode 0x60"));
        let read = read_class(&synthetic(&[class_file::ICONST_5])).unwrap();
        assert_eq!(read.definitions.len(), 1);
        let mut empty = synthetic(&[]);
        empty.methods[0].code = Some(vec![RETURN]);
        assert!(read_class(&empty).is_err());
        empty.methods.clear();
        assert!(read_class(&empty).is_err());
    }

    #[test]
    fn a_library_without_a_jar_reads_nothing_and_says_why() {
        let library = AnimationLibrary::new(None);
        let error = library.class("WardenAnimation").unwrap_err();
        assert!(format!("{error}").contains("no client jar"));
        assert!(library.definition("BatAnimation", "BAT_FLYING").is_none());
        assert_eq!(
            class_path("WardenAnimation"),
            "net/minecraft/client/animation/definitions/WardenAnimation"
        );
        assert_eq!(class_path("a/B"), "a/B");
    }

    fn jar() -> Option<PathBuf> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = Path::new(&root).join("client.jar");
        jar.is_file().then_some(jar)
    }

    #[test]
    fn the_warden_reads_its_animations_from_the_jar() {
        let Some(jar) = jar() else {
            return;
        };
        let library = AnimationLibrary::new(Some(jar));
        let warden = library.class("WardenAnimation").unwrap();
        let expected = [
            ("WARDEN_EMERGE", 6.68, 12, 200),
            ("WARDEN_DIG", 5.0, 12, 108),
            ("WARDEN_ROAR", 4.2, 8, 51),
            ("WARDEN_SNIFF", 4.16, 4, 25),
            ("WARDEN_ATTACK", 0.33333, 8, 32),
            ("WARDEN_SONIC_BOOM", 3.0, 10, 66),
        ];
        assert_eq!(warden.definitions.len(), expected.len());
        for (name, length, channels, keyframes) in expected {
            let definition = warden.get(name).unwrap();
            assert_eq!(definition.length, length, "{name}");
            assert!(!definition.looping, "{name}");
            assert_eq!(definition.channel_count(), channels, "{name}");
            assert_eq!(definition.keyframe_count(), keyframes, "{name}");
        }
        let catmullrom = warden
            .definitions
            .iter()
            .flat_map(|(_, definition)| &definition.bones)
            .flat_map(|(_, channels)| channels)
            .flat_map(|channel| &channel.keyframes)
            .filter(|keyframe| keyframe.interpolation == Interpolation::CatmullRom)
            .count();
        assert_eq!(catmullrom, 473);
        let emerge = warden.get("WARDEN_EMERGE").unwrap();
        let body = emerge.channel("body", Target::Rotation).unwrap();
        assert_eq!(body.keyframes.len(), 16);
        assert_eq!(
            body.keyframes[1],
            Keyframe::new(0.52, degree_vec(0.0, 0.0, -22.5), Interpolation::CatmullRom)
        );
        let mut bones: Vec<&str> = warden
            .definitions
            .iter()
            .flat_map(|(_, definition)| &definition.bones)
            .map(|(bone, _)| bone.as_str())
            .collect();
        bones.sort_unstable();
        bones.dedup();
        assert_eq!(
            bones,
            [
                "body",
                "head",
                "left_arm",
                "left_leg",
                "left_ribcage",
                "right_arm",
                "right_leg",
                "right_ribcage"
            ]
        );
        let again = library.class("WardenAnimation").unwrap();
        assert!(Arc::ptr_eq(&warden, &again));
        assert!(
            library
                .definition("WardenAnimation", "WARDEN_SONIC_BOOM")
                .is_some()
        );
        assert!(library.class("NoSuchAnimation").is_err());
    }

    #[test]
    fn the_bat_reads_its_animations_from_the_jar() {
        let Some(jar) = jar() else {
            return;
        };
        let bat = read_from_jar(&jar, "BatAnimation").unwrap();
        let resting = bat.get("BAT_RESTING").unwrap();
        let flying = bat.get("BAT_FLYING").unwrap();
        for definition in [resting, flying] {
            assert_eq!(definition.length, 0.5);
            assert!(definition.looping);
        }
        assert_eq!(resting.channel_count(), 11);
        assert_eq!(flying.channel_count(), 9);
        assert_eq!(resting.keyframe_count() + flying.keyframe_count(), 51);
        assert_eq!(
            resting.channel("head", Target::Rotation).unwrap().keyframes,
            vec![Keyframe::new(
                0.0,
                degree_vec(180.0, 0.0, 0.0),
                Interpolation::Linear
            )]
        );
    }

    #[test]
    fn every_animation_class_in_the_jar_reads() {
        let Some(jar) = jar() else {
            return;
        };
        let file = std::fs::File::open(&jar).unwrap();
        let archive = zip::ZipArchive::new(file).unwrap();
        let names: Vec<String> = archive
            .file_names()
            .filter_map(|name| name.strip_prefix(DEFINITIONS)?.strip_suffix(".class"))
            .filter(|name| name.ends_with("Animation"))
            .map(str::to_owned)
            .collect();
        assert!(names.len() >= 16, "{names:?}");
        let library = AnimationLibrary::new(Some(jar));
        for name in &names {
            let class = library.class(name).unwrap();
            for (field, definition) in &class.definitions {
                assert!(definition.channel_count() > 0, "{name}.{field}");
                assert!(definition.length >= 0.0, "{name}.{field}");
            }
        }
    }
}
