use minecraftoss_player::GameMode;
use minecraftoss_world::book;
use minecraftoss_world::game_rules::{self, Rule, Value};
use minecraftoss_world::item_components;

use crate::reader::{CommandError, StringReader, literal};
use crate::spawnpoint::java_float;

pub const TICKS_PER_DAY: i32 = 24_000;
pub const TIME_MARKERS: [(&str, i32); 6] = [
    ("day", 1000),
    ("midnight", 18_000),
    ("night", 13_000),
    ("noon", 6000),
    ("roll_village_siege", 18_000),
    ("wake_up_from_sleep", 0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Registry {
    EntityType,
    MobEffect,
    Enchantment,
    Structure,
    Biome,
    PointOfInterest,
    Dimension,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ArgKind {
    Players { single: bool },
    Time { min: i32 },
    Rule,
    RuleValue,
    TimeMarker,
    GameMode,
    Greedy,
    Int { min: i32, max: i32 },
    Bool,
    Item,
    Vec3,
    Resource(Registry),
    Nbt,
    ItemPredicate,
    BlockPos,
    BlockState,
    BlockPredicate,
    Rotation,
    ResourceOrTag(Registry),
    Float { min: f32, max: f32 },
    Word,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ItemArg {
    pub id: String,
    pub enchantments: Vec<(String, i32)>,
    pub potion: Option<String>,
    pub at: usize,
    pub components: Option<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemPredicate {
    Any,
    Item(String),
    Tag(String),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WorldCoordinate {
    pub value: f64,
    pub relative: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Coordinates {
    World([WorldCoordinate; 3]),
    Local([f64; 3]),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Selector {
    Me,
    Nearest,
    All,
    Random,
    Name(String),
}

#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    Players(Selector),
    Ticks(i32),
    Rule(&'static Rule),
    RuleValue(Value),
    Marker(String),
    GameMode(GameMode),
    Text(String),
    Int(i32),
    Bool(bool),
    Item(ItemArg),
    Vec3(Coordinates),
    Resource { id: String, at: usize },
    Nbt(String),
    ItemPredicate { predicate: ItemPredicate, at: usize },
    BlockPos(Coordinates),
    Block(crate::blocks::ParsedBlock),
    Rotation([WorldCoordinate; 2]),
    ResourceOrTag {
        id: String,
        tag: bool,
        at: usize,
        end: usize,
    },
    Float(f32),
}

impl ArgKind {
    pub fn parse(
        self,
        reader: &mut StringReader,
        parsed: &[(&'static str, Arg)],
    ) -> Result<Arg, CommandError> {
        match self {
            Self::Players { single } => players(reader, single).map(Arg::Players),
            Self::Time { min } => time(reader, min).map(Arg::Ticks),
            Self::Rule => {
                let start = reader.cursor;
                let id = reader.read_word();
                game_rules::rule(id).map(Arg::Rule).ok_or_else(|| {
                    reader.cursor = start;
                    CommandError::new("command.unknown.argument", Vec::new()).at(start)
                })
            }
            Self::RuleValue => {
                let rule = parsed
                    .iter()
                    .rev()
                    .find_map(|(_, arg)| match arg {
                        Arg::Rule(rule) => Some(*rule),
                        _ => None,
                    })
                    .ok_or_else(|| {
                        CommandError::new("command.unknown.argument", Vec::new()).at(reader.cursor)
                    })?;
                rule_value(reader, rule).map(Arg::RuleValue)
            }
            Self::TimeMarker => {
                let start = reader.cursor;
                let id = reader.read_word();
                let valid = !id.is_empty()
                    && id.matches(':').count() <= 1
                    && id
                        .chars()
                        .all(|c| c == ':' || c == '/' || crate::reader::allowed_in_unquoted(c));
                if valid {
                    Ok(Arg::Marker(if id.contains(':') {
                        id.to_owned()
                    } else {
                        format!("minecraft:{id}")
                    }))
                } else {
                    reader.cursor = start;
                    Err(CommandError::new("argument.id.invalid", Vec::new()).at(start))
                }
            }
            Self::GameMode => {
                let name = reader.read_unquoted();
                crate::players::game_mode_named(name)
                    .map(Arg::GameMode)
                    .ok_or_else(|| {
                        CommandError::new("argument.gamemode.invalid", vec![literal(name)])
                            .at(reader.cursor)
                    })
            }
            Self::Greedy => {
                let rest = reader.remaining().to_owned();
                reader.cursor = reader.input().len();
                Ok(Arg::Text(rest))
            }
            Self::Word => Ok(Arg::Text(reader.read_unquoted().to_owned())),
            Self::Int { min, max } => reader.read_bounded_int(min, max).map(Arg::Int),
            Self::Bool => reader.read_bool().map(Arg::Bool),
            Self::Item => item(reader).map(Arg::Item),
            Self::Vec3 => vec3(reader).map(Arg::Vec3),
            Self::Resource(_) => resource_id(reader).map(|id| Arg::Resource {
                id,
                at: reader.cursor,
            }),
            Self::Nbt => nbt(reader).map(Arg::Nbt),
            Self::ItemPredicate => {
                let at = reader.cursor;
                let predicate = match reader.peek() {
                    Some('*') => {
                        reader.skip();
                        ItemPredicate::Any
                    }
                    Some('#') => {
                        reader.skip();
                        ItemPredicate::Tag(resource_id(reader)?)
                    }
                    _ => ItemPredicate::Item(resource_id(reader)?),
                };
                Ok(Arg::ItemPredicate { predicate, at })
            }
            Self::BlockPos => block_pos(reader).map(Arg::BlockPos),
            Self::BlockState => crate::blocks::parse_block(reader, false).map(Arg::Block),
            Self::BlockPredicate => crate::blocks::parse_block(reader, true).map(Arg::Block),
            Self::Rotation => rotation(reader).map(Arg::Rotation),
            Self::ResourceOrTag(_) => {
                let at = reader.cursor;
                let tag = reader.peek() == Some('#');
                if tag {
                    reader.skip();
                }
                let id = resource_id(reader).inspect_err(|_| reader.cursor = at)?;
                Ok(Arg::ResourceOrTag {
                    id,
                    tag,
                    at,
                    end: reader.cursor,
                })
            }
            Self::Float { min, max } => bounded_float(reader, min, max).map(Arg::Float),
        }
    }
}

fn bounded_float(reader: &mut StringReader, min: f32, max: f32) -> Result<f32, CommandError> {
    let start = reader.cursor;
    let value = reader.read_float()?;
    let bound = if value < min {
        Some(("argument.float.low", min))
    } else if value > max {
        Some(("argument.float.big", max))
    } else {
        None
    };
    if let Some((key, bound)) = bound {
        reader.cursor = start;
        return Err(CommandError::new(
            key,
            vec![literal(java_float(bound)), literal(java_float(value))],
        )
        .at(start));
    }
    Ok(value)
}

pub fn marker_ticks(id: &str) -> Option<i32> {
    let path = id.strip_prefix("minecraft:")?;
    TIME_MARKERS
        .iter()
        .find(|(name, _)| *name == path)
        .map(|(_, ticks)| *ticks)
}

fn rule_value(reader: &mut StringReader, rule: &'static Rule) -> Result<Value, CommandError> {
    match rule.default {
        Value::Bool(_) => reader.read_bool().map(Value::Bool),
        Value::Int(_) => reader.read_bounded_int(rule.min, rule.max).map(Value::Int),
    }
}

pub fn time(reader: &mut StringReader, min: i32) -> Result<i32, CommandError> {
    let amount = reader.read_float()?;
    let unit_at = reader.cursor;
    let scale = match reader.read_unquoted() {
        "" | "t" => 1.0,
        "s" => 20.0,
        "d" => TICKS_PER_DAY as f32,
        _ => {
            reader.cursor = unit_at;
            return Err(CommandError::new("argument.time.invalid_unit", Vec::new()).at(unit_at));
        }
    };
    let ticks = (amount * scale).round() as i32;
    if ticks < min {
        return Err(CommandError::new(
            "argument.time.tick_count_too_low",
            vec![literal(min), literal(ticks)],
        )
        .at(reader.cursor));
    }
    Ok(ticks)
}

fn players(reader: &mut StringReader, single: bool) -> Result<Selector, CommandError> {
    let start = reader.cursor;
    if reader.peek() == Some('@') {
        reader.skip();
        let Some(kind) = reader.peek() else {
            return Err(
                CommandError::new("argument.entity.selector.missing", Vec::new()).at(start),
            );
        };
        reader.skip();
        let selector = match kind {
            's' => Selector::Me,
            'p' => Selector::Nearest,
            'a' => Selector::All,
            'r' => Selector::Random,
            'e' | 'n' => {
                reader.cursor = start;
                return Err(
                    CommandError::new("argument.entity.selector.not_allowed", Vec::new()).at(start),
                );
            }
            other => {
                reader.cursor = start;
                return Err(CommandError::new(
                    "argument.entity.selector.unknown",
                    vec![literal(format!("@{other}"))],
                )
                .at(start));
            }
        };
        if reader.peek() == Some('[') {
            let option_at = reader.cursor + 1;
            reader.skip();
            let option = reader.read_unquoted();
            reader.cursor = option_at;
            return Err(CommandError::new(
                "argument.entity.options.unknown",
                vec![literal(option)],
            )
            .at(option_at));
        }
        if single && selector == Selector::All {
            reader.cursor = start;
            return Err(CommandError::new("argument.player.toomany", Vec::new()).at(start));
        }
        return Ok(selector);
    }
    let name = reader.read_string()?;
    if name.is_empty() || name.chars().count() > 16 {
        reader.cursor = start;
        return Err(CommandError::new("argument.entity.invalid", Vec::new()).at(start));
    }
    Ok(Selector::Name(name))
}

pub const ENCHANTMENT_COMPONENTS: [&str; 2] =
    ["minecraft:enchantments", "minecraft:stored_enchantments"];
pub const OMINOUS_AMPLIFIER: &str = "minecraft:ominous_bottle_amplifier";
pub const CUSTOM_DATA: &str = "minecraft:custom_data";

pub(crate) fn allowed_in_id(c: char) -> bool {
    c.is_ascii_digit() || c.is_ascii_lowercase() || matches!(c, '_' | ':' | '/' | '.' | '-')
}

pub fn resource_id(reader: &mut StringReader) -> Result<String, CommandError> {
    let start = reader.cursor;
    while reader.peek().is_some_and(allowed_in_id) {
        reader.skip();
    }
    let text = &reader.input()[start..reader.cursor];
    let (namespace, path) = match text.split_once(':') {
        Some(("", path)) => ("minecraft", path),
        Some(split) => split,
        None => ("minecraft", text),
    };
    if namespace.contains('/') || path.contains(':') {
        reader.cursor = start;
        return Err(CommandError::new("argument.id.invalid", Vec::new()).at(start));
    }
    Ok(format!("{namespace}:{path}"))
}

fn item(reader: &mut StringReader) -> Result<ItemArg, CommandError> {
    let at = reader.cursor;
    let id = resource_id(reader)?;
    let mut enchantments = Vec::new();
    let mut books = serde_json::Map::new();
    match reader.peek() {
        Some('[') => components(reader, &mut enchantments, &mut books)?,
        Some('+') => kit_enchantments(reader, &mut enchantments)?,
        _ => {}
    }
    Ok(ItemArg {
        id,
        enchantments,
        potion: None,
        at,
        components: (!books.is_empty()).then_some(serde_json::Value::Object(books)),
    })
}

fn components(
    reader: &mut StringReader,
    out: &mut Vec<(String, i32)>,
    books: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommandError> {
    reader.expect('[')?;
    let mut seen: Vec<String> = Vec::new();
    while reader.can_read() && reader.peek() != Some(']') {
        reader.skip_ws();
        let start = reader.cursor;
        let id = resource_id(reader)?;
        let book = book::CONTENT_COMPONENTS.contains(&id.as_str());
        let ominous = id == OMINOUS_AMPLIFIER;
        let custom = id == CUSTOM_DATA;
        let listed = item_components::COMPONENTS.contains(&id.as_str());
        if !book
            && !ominous
            && !custom
            && !listed
            && !ENCHANTMENT_COMPONENTS.contains(&id.as_str())
        {
            reader.cursor = start;
            return Err(
                CommandError::new("arguments.item.component.unknown", vec![literal(&id)]).at(start),
            );
        }
        if seen.contains(&id) {
            return Err(CommandError::new(
                "arguments.item.component.repeated",
                vec![literal(&id)],
            ));
        }
        reader.skip_ws();
        reader.expect('=')?;
        reader.skip_ws();
        if book {
            book_content(reader, &id, books)?;
        } else if ominous {
            ominous_amplifier(reader, &id, books)?;
        } else if custom {
            custom_data(reader, books)?;
        } else if listed {
            item_component(reader, &id, books)?;
        } else {
            enchantment_levels(reader, &id, out)?;
        }
        seen.push(id);
        reader.skip_ws();
        if reader.peek() != Some(',') {
            break;
        }
        reader.skip();
        reader.skip_ws();
        if !reader.can_read() {
            return Err(
                CommandError::new("arguments.item.component.expected", Vec::new())
                    .at(reader.cursor),
            );
        }
    }
    reader.expect(']')
}

fn book_content(
    reader: &mut StringReader,
    component: &str,
    books: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommandError> {
    let start = reader.cursor;
    let text = nbt(reader)?;
    match book::content(component, &text) {
        Ok(content) => {
            books.insert(component.to_owned(), content);
            Ok(())
        }
        Err(reason) => {
            reader.cursor = start;
            Err(CommandError::new(
                "arguments.item.component.malformed",
                vec![literal(component), literal(reason)],
            )
            .at(start))
        }
    }
}

fn ominous_amplifier(
    reader: &mut StringReader,
    component: &str,
    components: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommandError> {
    let start = reader.cursor;
    let amplifier = reader.read_int()?;
    if !(0..=4).contains(&amplifier) {
        reader.cursor = start;
        return Err(CommandError::new(
            "arguments.item.component.malformed",
            vec![
                literal(component),
                literal(format!("Value {amplifier} outside of range [0:4]")),
            ],
        )
        .at(start));
    }
    components.insert(component.to_owned(), serde_json::json!(amplifier));
    Ok(())
}

fn custom_data(
    reader: &mut StringReader,
    books: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommandError> {
    let start = reader.cursor;
    let text = nbt(reader)?;
    match minecraftoss_core::snbt::parse_compound(&text) {
        Ok(tag) => {
            books.insert(CUSTOM_DATA.to_owned(), tag_json(&tag));
            Ok(())
        }
        Err(reason) => {
            reader.cursor = start;
            Err(CommandError::new(
                "arguments.item.component.malformed",
                vec![literal(CUSTOM_DATA), literal(reason)],
            )
            .at(start))
        }
    }
}

fn item_component(
    reader: &mut StringReader,
    component: &str,
    components: &mut serde_json::Map<String, serde_json::Value>,
) -> Result<(), CommandError> {
    let start = reader.cursor;
    let text = snbt_value(reader);
    match item_components::content(component, &text) {
        Ok(value) => {
            components.insert(component.to_owned(), value);
            Ok(())
        }
        Err(reason) => {
            reader.cursor = start;
            Err(CommandError::new(
                "arguments.item.component.malformed",
                vec![literal(component), literal(reason)],
            )
            .at(start))
        }
    }
}

fn snbt_value(reader: &mut StringReader) -> String {
    let start = reader.cursor;
    let mut depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    while let Some(c) = reader.peek() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                ',' | ']' if depth == 0 => break,
                '"' | '\'' => quote = Some(c),
                '{' | '[' => depth += 1,
                '}' | ']' => depth = depth.saturating_sub(1),
                _ => {}
            },
        }
        reader.skip();
    }
    reader.input()[start..reader.cursor].to_owned()
}

fn tag_json(tag: &minecraftoss_core::nbt::Tag) -> serde_json::Value {
    use minecraftoss_core::nbt::Tag;
    use serde_json::Value;
    match tag {
        Tag::Byte(v) => Value::from(*v),
        Tag::Short(v) => Value::from(*v),
        Tag::Int(v) => Value::from(*v),
        Tag::Long(v) => Value::from(*v),
        Tag::Float(v) => Value::from(f64::from(*v)),
        Tag::Double(v) => Value::from(*v),
        Tag::String(v) => Value::from(v.clone()),
        Tag::ByteArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::IntArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::LongArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::List(list) => Value::Array(list.iter().map(tag_json).collect()),
        Tag::Compound(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), tag_json(value)))
                .collect(),
        ),
    }
}

fn enchantment_levels(
    reader: &mut StringReader,
    component: &str,
    out: &mut Vec<(String, i32)>,
) -> Result<(), CommandError> {
    let start = reader.cursor;
    reader.expect('{')?;
    reader.skip_ws();
    while reader.can_read() && reader.peek() != Some('}') {
        let key_at = reader.cursor;
        let key = reader.read_string()?;
        if key.is_empty() {
            return Err(CommandError::new("argument.nbt.expected.key", Vec::new()).at(key_at));
        }
        let id = if key.contains(':') {
            key
        } else {
            format!("minecraft:{key}")
        };
        reader.skip_ws();
        reader.expect(':')?;
        reader.skip_ws();
        let level = reader.read_int()?;
        if reader
            .peek()
            .is_some_and(|c| matches!(c, 'b' | 'B' | 's' | 'S' | 'i' | 'I' | 'l' | 'L'))
        {
            reader.skip();
        }
        if !(1..=255).contains(&level) {
            reader.cursor = start;
            return Err(CommandError::new(
                "arguments.item.component.malformed",
                vec![
                    literal(component),
                    literal(format!("Value {level} outside of range [1:255]")),
                ],
            )
            .at(start));
        }
        out.retain(|(known, _)| *known != id);
        out.push((id, level));
        reader.skip_ws();
        if reader.peek() != Some(',') {
            break;
        }
        reader.skip();
        reader.skip_ws();
    }
    reader.expect('}')
}

fn kit_enchantments(
    reader: &mut StringReader,
    out: &mut Vec<(String, i32)>,
) -> Result<(), CommandError> {
    while reader.peek() == Some('+') {
        reader.skip();
        let start = reader.cursor;
        let id = resource_id(reader)?;
        if id.ends_with(':') {
            return Err(CommandError::new("argument.id.invalid", Vec::new()).at(start));
        }
        let level = if reader.peek() == Some('=') {
            reader.skip();
            reader.read_int()?
        } else {
            1
        };
        out.push((id, level));
    }
    Ok(())
}

fn incomplete(reader: &mut StringReader, start: usize) -> CommandError {
    reader.cursor = start;
    CommandError::new("argument.pos3d.incomplete", Vec::new()).at(start)
}

fn world_coordinate(
    reader: &mut StringReader,
    center: bool,
) -> Result<WorldCoordinate, CommandError> {
    if reader.peek() == Some('^') {
        return Err(CommandError::new("argument.pos.mixed", Vec::new()).at(reader.cursor));
    }
    if !reader.can_read() {
        return Err(CommandError::new("argument.pos.missing.double", Vec::new()).at(reader.cursor));
    }
    let relative = reader.peek() == Some('~');
    if relative {
        reader.skip();
    }
    let start = reader.cursor;
    let value = if reader.can_read() && reader.peek() != Some(' ') {
        reader.read_double()?
    } else {
        0.0
    };
    let centered = center && !relative && !reader.input()[start..reader.cursor].contains('.');
    Ok(WorldCoordinate {
        value: if centered { value + 0.5 } else { value },
        relative,
    })
}

fn local_coordinate(reader: &mut StringReader, start: usize) -> Result<f64, CommandError> {
    if !reader.can_read() {
        return Err(CommandError::new("argument.pos.missing.double", Vec::new()).at(reader.cursor));
    }
    if reader.peek() != Some('^') {
        reader.cursor = start;
        return Err(CommandError::new("argument.pos.mixed", Vec::new()).at(start));
    }
    reader.skip();
    if reader.can_read() && reader.peek() != Some(' ') {
        reader.read_double()
    } else {
        Ok(0.0)
    }
}

fn vec3(reader: &mut StringReader) -> Result<Coordinates, CommandError> {
    let start = reader.cursor;
    let local = reader.peek() == Some('^');
    let mut world = [WorldCoordinate {
        value: 0.0,
        relative: false,
    }; 3];
    let mut offsets = [0.0; 3];
    for axis in 0..3 {
        if axis > 0 {
            if reader.peek() != Some(' ') {
                return Err(incomplete(reader, start));
            }
            reader.skip();
        }
        if local {
            offsets[axis] = local_coordinate(reader, start)?;
        } else {
            world[axis] = world_coordinate(reader, axis != 1)?;
        }
    }
    Ok(if local {
        Coordinates::Local(offsets)
    } else {
        Coordinates::World(world)
    })
}

fn int_coordinate(reader: &mut StringReader) -> Result<WorldCoordinate, CommandError> {
    if reader.peek() == Some('^') {
        return Err(CommandError::new("argument.pos.mixed", Vec::new()).at(reader.cursor));
    }
    if !reader.can_read() {
        return Err(CommandError::new("argument.pos.missing.int", Vec::new()).at(reader.cursor));
    }
    let relative = reader.peek() == Some('~');
    if relative {
        reader.skip();
    }
    let value = if !reader.can_read() || reader.peek() == Some(' ') {
        0.0
    } else if relative {
        reader.read_double()?
    } else {
        f64::from(reader.read_int()?)
    };
    Ok(WorldCoordinate { value, relative })
}

fn rotation(reader: &mut StringReader) -> Result<[WorldCoordinate; 2], CommandError> {
    let start = reader.cursor;
    let incomplete = |reader: &mut StringReader| {
        reader.cursor = start;
        CommandError::new("argument.rotation.incomplete", Vec::new()).at(start)
    };
    if !reader.can_read() {
        return Err(incomplete(reader));
    }
    let yaw = world_coordinate(reader, false)?;
    if reader.peek() != Some(' ') {
        return Err(incomplete(reader));
    }
    reader.skip();
    let pitch = world_coordinate(reader, false)?;
    Ok([yaw, pitch])
}

fn block_pos(reader: &mut StringReader) -> Result<Coordinates, CommandError> {
    let start = reader.cursor;
    if reader.peek() == Some('^') {
        let mut offsets = [0.0; 3];
        for (axis, offset) in offsets.iter_mut().enumerate() {
            if axis > 0 {
                if reader.peek() != Some(' ') {
                    return Err(incomplete(reader, start));
                }
                reader.skip();
            }
            *offset = local_coordinate(reader, start)?;
        }
        return Ok(Coordinates::Local(offsets));
    }
    let mut axes = [WorldCoordinate {
        value: 0.0,
        relative: false,
    }; 3];
    for (axis, coordinate) in axes.iter_mut().enumerate() {
        if axis > 0 {
            if reader.peek() != Some(' ') {
                return Err(incomplete(reader, start));
            }
            reader.skip();
        }
        *coordinate = int_coordinate(reader)?;
    }
    Ok(Coordinates::World(axes))
}

impl Coordinates {
    pub fn position(&self, pose: crate::server::Pose) -> [f64; 3] {
        match *self {
            Self::World(axes) => std::array::from_fn(|i| {
                axes[i].value + if axes[i].relative { pose.feet[i] } else { 0.0 }
            }),
            Self::Local([left, up, forwards]) => {
                let radians = std::f32::consts::PI / 180.0;
                let (yaw, pitch) = (pose.yaw + 90.0, -pose.pitch);
                let (f, g) = ((yaw * radians).cos(), (yaw * radians).sin());
                let (h, i) = ((pitch * radians).cos(), (pitch * radians).sin());
                let (j, k) = (
                    ((pitch + 90.0) * radians).cos(),
                    ((pitch + 90.0) * radians).sin(),
                );
                let ahead = [f * h, i, g * h].map(f64::from);
                let above = [f * j, k, g * j].map(f64::from);
                let side = [
                    above[1] * ahead[2] - above[2] * ahead[1],
                    above[2] * ahead[0] - above[0] * ahead[2],
                    above[0] * ahead[1] - above[1] * ahead[0],
                ];
                std::array::from_fn(|n| {
                    pose.feet[n] + ahead[n] * forwards + above[n] * up + side[n] * left
                })
            }
        }
    }
}

pub(crate) fn nbt(reader: &mut StringReader) -> Result<String, CommandError> {
    let start = reader.cursor;
    if reader.peek() != Some('{') {
        return Err(CommandError::new("argument.nbt.expected.compound", Vec::new()).at(start));
    }
    let mut depth = 0_usize;
    let mut quote = None;
    let mut escaped = false;
    while let Some(c) = reader.peek() {
        reader.skip();
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '{' | '[' => depth += 1,
                '}' | ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Ok(reader.input()[start..reader.cursor].to_owned());
                    }
                }
                _ => {}
            },
        }
    }
    Err(CommandError::new("parsing.expected", vec![literal('}')]).at(reader.cursor))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ticks(text: &str) -> Result<i32, CommandError> {
        time(&mut StringReader::new(text), 0)
    }

    fn selector(text: &str, single: bool) -> Result<Selector, CommandError> {
        players(&mut StringReader::new(text), single)
    }

    fn key(error: &CommandError) -> String {
        match &error.text.content {
            crate::text::Content::Translate { key, .. } => key.clone(),
            _ => String::new(),
        }
    }

    #[test]
    fn time_units_scale_ticks() {
        assert_eq!(ticks("1d"), Ok(24_000));
        assert_eq!(ticks("10s"), Ok(200));
        assert_eq!(ticks("5t"), Ok(5));
        assert_eq!(ticks("7"), Ok(7));
        assert_eq!(ticks("0.5d"), Ok(12_000));
        assert_eq!(
            key(&ticks("-1").unwrap_err()),
            "argument.time.tick_count_too_low"
        );
        assert_eq!(key(&ticks("3w").unwrap_err()), "argument.time.invalid_unit");
        assert_eq!(time(&mut StringReader::new("-1"), i32::MIN), Ok(-1));
        let short = time(&mut StringReader::new("0"), 1).unwrap_err();
        assert_eq!(
            short.text,
            crate::text::Text::translate(
                "argument.time.tick_count_too_low",
                vec![literal(1), literal(0)]
            )
        );
    }

    #[test]
    fn selectors_and_names_parse_like_vanilla() {
        assert_eq!(selector("@s", false), Ok(Selector::Me));
        assert_eq!(selector("@p", true), Ok(Selector::Nearest));
        assert_eq!(selector("@a", false), Ok(Selector::All));
        assert_eq!(selector("@r", true), Ok(Selector::Random));
        assert_eq!(selector("Steve", true), Ok(Selector::Name("Steve".into())));
        assert_eq!(
            selector("\"Mr Steve\"", true),
            Ok(Selector::Name("Mr Steve".into()))
        );
        assert_eq!(
            key(&selector("@a", true).unwrap_err()),
            "argument.player.toomany"
        );
        assert_eq!(
            key(&selector("@e", false).unwrap_err()),
            "argument.entity.selector.not_allowed"
        );
        assert_eq!(
            key(&selector("@x", false).unwrap_err()),
            "argument.entity.selector.unknown"
        );
        assert_eq!(
            key(&selector("@a[limit=1]", false).unwrap_err()),
            "argument.entity.options.unknown"
        );
        assert_eq!(
            key(&selector("abcdefghijklmnopq", false).unwrap_err()),
            "argument.entity.invalid"
        );
    }

    #[test]
    fn rules_parse_short_and_namespaced_ids_and_their_values() {
        let parse = |text: &str, kind: ArgKind, parsed: &[(&'static str, Arg)]| {
            kind.parse(&mut StringReader::new(text), parsed)
        };
        let Ok(Arg::Rule(rule)) = parse("minecraft:keep_inventory", ArgKind::Rule, &[]) else {
            panic!("namespaced rule");
        };
        assert_eq!(rule.id, "keep_inventory");
        assert!(matches!(
            parse("keep_inventory", ArgKind::Rule, &[]),
            Ok(Arg::Rule(_))
        ));
        assert_eq!(
            key(&parse("keepInventory", ArgKind::Rule, &[]).unwrap_err()),
            "command.unknown.argument"
        );
        let with = [("rule", Arg::Rule(rule))];
        assert_eq!(
            parse("true", ArgKind::RuleValue, &with),
            Ok(Arg::RuleValue(Value::Bool(true)))
        );
        assert_eq!(
            key(&parse("1", ArgKind::RuleValue, &with).unwrap_err()),
            "parsing.bool.invalid"
        );
        let snow = [(
            "rule",
            Arg::Rule(game_rules::rule("max_snow_accumulation_height").unwrap()),
        )];
        assert_eq!(
            key(&parse("9", ArgKind::RuleValue, &snow).unwrap_err()),
            "argument.integer.big"
        );
        assert_eq!(
            parse("8", ArgKind::RuleValue, &snow),
            Ok(Arg::RuleValue(Value::Int(8)))
        );
    }

    #[test]
    fn game_modes_parse_their_four_names() {
        let parse = |text: &str| ArgKind::GameMode.parse(&mut StringReader::new(text), &[]);
        assert_eq!(parse("creative"), Ok(Arg::GameMode(GameMode::Creative)));
        assert_eq!(parse("spectator"), Ok(Arg::GameMode(GameMode::Spectator)));
        let error = parse("god").unwrap_err();
        assert_eq!(key(&error), "argument.gamemode.invalid");
        assert_eq!(error.cursor, Some(3));
        assert_eq!(key(&parse("1").unwrap_err()), "argument.gamemode.invalid");
    }

    #[test]
    fn time_markers_take_the_minecraft_namespace() {
        let parse = |text: &str| ArgKind::TimeMarker.parse(&mut StringReader::new(text), &[]);
        assert_eq!(parse("noon"), Ok(Arg::Marker("minecraft:noon".into())));
        assert_eq!(
            parse("minecraft:midnight"),
            Ok(Arg::Marker("minecraft:midnight".into()))
        );
        assert_eq!(marker_ticks("minecraft:noon"), Some(6000));
        assert_eq!(marker_ticks("minecraft:day"), Some(1000));
        assert_eq!(marker_ticks("minecraft:dusk"), None);
        assert_eq!(marker_ticks("other:noon"), None);
        assert!(parse("a:b:c").is_err());
    }

    fn parse_item(text: &str) -> Result<ItemArg, CommandError> {
        match ArgKind::Item.parse(&mut StringReader::new(text), &[])? {
            Arg::Item(item) => Ok(item),
            other => panic!("{other:?}"),
        }
    }

    fn levels(pairs: &[(&str, i32)]) -> Vec<(String, i32)> {
        pairs
            .iter()
            .map(|(id, level)| ((*id).to_owned(), *level))
            .collect()
    }

    #[test]
    fn items_parse_vanilla_enchantment_components_and_kit_plus_syntax() {
        let sword =
            parse_item("diamond_sword[enchantments={sharpness:5,\"minecraft:looting\":3b}]")
                .unwrap();
        assert_eq!(sword.id, "minecraft:diamond_sword");
        assert_eq!(
            sword.enchantments,
            levels(&[("minecraft:sharpness", 5), ("minecraft:looting", 3)])
        );
        assert_eq!(sword.at, 0);
        let spaced =
            parse_item("minecraft:bow[ minecraft:enchantments = { power : 2 , flame:1 } ]")
                .unwrap();
        assert_eq!(
            spaced.enchantments,
            levels(&[("minecraft:power", 2), ("minecraft:flame", 1)])
        );
        let book = parse_item("enchanted_book[stored_enchantments={mending:1}]").unwrap();
        assert_eq!(book.enchantments, levels(&[("minecraft:mending", 1)]));
        let pickaxe = parse_item("diamond_pickaxe+silk_touch+minecraft:efficiency=5").unwrap();
        assert_eq!(pickaxe.id, "minecraft:diamond_pickaxe");
        assert_eq!(
            pickaxe.enchantments,
            levels(&[("minecraft:silk_touch", 1), ("minecraft:efficiency", 5)])
        );
        let plain = parse_item("stone").unwrap();
        assert_eq!(
            (plain.id.as_str(), plain.enchantments.len()),
            ("minecraft:stone", 0)
        );
        assert_eq!(
            key(&parse_item("diamond_sword[custom_name='x']").unwrap_err()),
            "arguments.item.component.unknown"
        );
        assert_eq!(
            key(&parse_item("diamond_sword[enchantments={sharpness:300}]").unwrap_err()),
            "arguments.item.component.malformed"
        );
        assert_eq!(
            key(
                &parse_item("diamond_sword[enchantments={sharpness:1},enchantments={}]")
                    .unwrap_err()
            ),
            "arguments.item.component.repeated"
        );
        assert_eq!(
            key(&parse_item("diamond_sword[enchantments={sharpness:1}").unwrap_err()),
            "parsing.expected"
        );
        assert_eq!(
            key(&parse_item("diamond_sword[enchantments={sharpness:1},").unwrap_err()),
            "arguments.item.component.expected"
        );
    }

    #[test]
    fn a_written_book_takes_title_author_and_pages() {
        let item = parse_item(
            r#"written_book[written_book_content={title:"Test",author:"Owner",pages:["Page one, with a comma.",'Page two',{text:"Three",bold:1b}]}]"#,
        )
        .unwrap();
        assert_eq!(item.id, "minecraft:written_book");
        assert_eq!(
            item.components,
            Some(serde_json::json!({"minecraft:written_book_content": {
                "title": {"raw": "Test"},
                "author": "Owner",
                "pages": [{"raw": "Page one, with a comma."}, {"raw": "Page two"}, {"raw": {"text": "Three", "bold": 1}}]
            }}))
        );
        assert!(item.enchantments.is_empty());
        let both = parse_item(
            r#"written_book[enchantments={unbreaking:1},minecraft:written_book_content={title:"T",author:"A"}]"#,
        )
        .unwrap();
        assert_eq!(both.enchantments, levels(&[("minecraft:unbreaking", 1)]));
        assert_eq!(
            both.components,
            Some(serde_json::json!({"minecraft:written_book_content": {"title": {"raw": "T"}, "author": "A"}}))
        );
    }

    #[test]
    fn writable_pages_take_plain_and_raw_strings() {
        let item = parse_item(r#"writable_book[writable_book_content={pages:["one",{raw:"two"}]}]"#).unwrap();
        assert_eq!(
            item.components,
            Some(serde_json::json!({"minecraft:writable_book_content": {"pages": [{"raw": "one"}, {"raw": "two"}]}}))
        );
        assert_eq!(parse_item("writable_book").unwrap().components, None);
    }

    #[test]
    fn custom_data_is_parsed_as_snbt() {
        let item =
            parse_item(r#"iw4l:m4[minecraft:custom_data={iw4l:{attachments:["reflex"],clip:5b}}]"#)
                .unwrap();
        assert_eq!(item.id, "iw4l:m4");
        assert_eq!(
            item.components,
            Some(serde_json::json!({"minecraft:custom_data": {"iw4l": {"attachments": ["reflex"], "clip": 5}}}))
        );
        let short = parse_item("stone[custom_data={a:1}]").unwrap();
        assert_eq!(
            short.components,
            Some(serde_json::json!({"minecraft:custom_data": {"a": 1}}))
        );
        let text = "stone[custom_data={a:}]";
        let error = parse_item(text).unwrap_err();
        assert_eq!(key(&error), "arguments.item.component.malformed");
        assert_eq!(error.cursor, text.find('{'));
    }

    #[test]
    fn bad_book_content_is_malformed() {
        for text in [
            r#"written_book[written_book_content={author:"A"}]"#,
            r#"written_book[written_book_content={title:"T",author:"A",generation:9}]"#,
            r#"writable_book[writable_book_content={pages:[{text:"x"}]}]"#,
            r#"writable_book[writable_book_content={pages:[1b]}]"#,
        ] {
            let error = parse_item(text).unwrap_err();
            assert_eq!(key(&error), "arguments.item.component.malformed", "{text}");
            assert_eq!(error.cursor, text.find('{'), "{text}");
        }
        assert_eq!(
            key(&parse_item("writable_book[writable_book_content=[]]").unwrap_err()),
            "argument.nbt.expected.compound"
        );
        assert_eq!(
            key(&parse_item(r#"written_book[written_book_content={title:"T",author:"A"},written_book_content={title:"T",author:"A"}]"#).unwrap_err()),
            "arguments.item.component.repeated"
        );
    }

    #[test]
    fn an_ominous_bottle_takes_its_amplifier_from_zero_to_four() {
        let bottle = parse_item("ominous_bottle[minecraft:ominous_bottle_amplifier=4]").unwrap();
        assert_eq!(bottle.id, "minecraft:ominous_bottle");
        assert_eq!(
            bottle.components,
            Some(serde_json::json!({"minecraft:ominous_bottle_amplifier": 4}))
        );
        let text = "ominous_bottle[ominous_bottle_amplifier=5]";
        let error = parse_item(text).unwrap_err();
        assert_eq!(key(&error), "arguments.item.component.malformed");
        assert_eq!(error.cursor, text.find('5'));
        assert_eq!(
            key(&parse_item("ominous_bottle[ominous_bottle_amplifier=x]").unwrap_err()),
            "parsing.int.expected"
        );
    }

    #[test]
    fn banners_and_shields_take_patterns_and_a_base_colour() {
        let banner = parse_item(
            r#"white_banner[banner_patterns=[{pattern:"minecraft:creeper",color:"black"},{pattern:"border",color:red}]]"#,
        )
        .unwrap();
        assert_eq!(
            banner.components,
            Some(serde_json::json!({"minecraft:banner_patterns": [
                {"pattern": "minecraft:creeper", "color": "black"},
                {"pattern": "minecraft:border", "color": "red"}
            ]}))
        );
        let shield = parse_item(
            r#"shield[base_color=red, minecraft:banner_patterns = [{pattern:{asset_id:"cross",translation_key:"block.minecraft.banner.cross"},color:"yellow"}]]"#,
        )
        .unwrap();
        assert_eq!(
            shield.components,
            Some(serde_json::json!({
                "minecraft:base_color": "red",
                "minecraft:banner_patterns": [{
                    "pattern": {"asset_id": "minecraft:cross", "translation_key": "block.minecraft.banner.cross"},
                    "color": "yellow"
                }]
            }))
        );
        let quoted = parse_item(r#"shield[base_color="light_blue"]"#).unwrap();
        assert_eq!(
            quoted.components,
            Some(serde_json::json!({"minecraft:base_color": "light_blue"}))
        );
    }

    #[test]
    fn a_crossbow_takes_its_charged_projectiles() {
        let crossbow =
            parse_item(r#"crossbow[charged_projectiles=[{id:"minecraft:arrow"}],enchantments={multishot:1}]"#)
                .unwrap();
        assert_eq!(
            crossbow.components,
            Some(serde_json::json!({"minecraft:charged_projectiles": [{"id": "minecraft:arrow"}]}))
        );
        assert_eq!(crossbow.enchantments, levels(&[("minecraft:multishot", 1)]));
        let three = parse_item(r#"crossbow[charged_projectiles=[{id:"spectral_arrow",count:3}]]"#).unwrap();
        assert_eq!(
            three.components,
            Some(serde_json::json!({"minecraft:charged_projectiles": [{"id": "minecraft:spectral_arrow", "count": 3}]}))
        );
    }

    #[test]
    fn rockets_and_stars_take_their_firework_components() {
        let rocket = parse_item(
            "firework_rocket[fireworks={flight_duration:1,explosions:[{shape:large_ball,colors:[I;11743532],has_trail:1b}]}]",
        )
        .unwrap();
        assert_eq!(
            rocket.components,
            Some(serde_json::json!({"minecraft:fireworks": {
                "flight_duration": 1,
                "explosions": [{"shape": "large_ball", "colors": [11743532], "has_trail": 1}]
            }}))
        );
        let star = parse_item(
            "firework_star[firework_explosion={shape:creeper,colors:[I;1,2],fade_colors:[3],has_twinkle:true}]",
        )
        .unwrap();
        assert_eq!(
            star.components,
            Some(serde_json::json!({"minecraft:firework_explosion": {
                "shape": "creeper", "colors": [1, 2], "fade_colors": [3], "has_twinkle": 1
            }}))
        );
    }

    #[test]
    fn bad_dyes_shapes_and_lists_are_malformed_components() {
        for (text, bad) in [
            ("shield[base_color=purpl]", "purpl"),
            (r#"white_banner[banner_patterns=[{pattern:"creeper",color:"grey"}]]"#, "[{"),
            ("firework_star[firework_explosion={shape:square}]", "{"),
            (r#"white_banner[banner_patterns={pattern:"creeper",color:"black"}]"#, "{"),
            (r#"crossbow[charged_projectiles=[{id:"arrow"}"#, "[{"),
            (r#"crossbow[charged_projectiles=[{id:"arrow",count:0}]]"#, "[{"),
            ("firework_rocket[fireworks={explosions:[{shape:star,colors:[I;1}]}]", "{"),
        ] {
            let error = parse_item(text).unwrap_err();
            assert_eq!(key(&error), "arguments.item.component.malformed", "{text}");
            assert_eq!(error.cursor, text.find(bad), "{text}");
        }
        assert_eq!(
            key(&parse_item("shield[base_color=red,base_color=blue]").unwrap_err()),
            "arguments.item.component.repeated"
        );
    }

    #[test]
    fn item_ids_take_vanilla_resource_location_syntax() {
        let mut reader = StringReader::new("Stone");
        let item = ArgKind::Item.parse(&mut reader, &[]).unwrap();
        assert_eq!(
            item,
            Arg::Item(ItemArg {
                id: "minecraft:".into(),
                enchantments: Vec::new(),
                potion: None,
                at: 0,
                components: None
            })
        );
        assert_eq!(reader.cursor, 0);
        let error = parse_item("a:b:c").unwrap_err();
        assert_eq!(
            (key(&error), error.cursor),
            ("argument.id.invalid".into(), Some(0))
        );
        assert_eq!(parse_item("mod:thing").unwrap().id, "mod:thing");
        assert_eq!(parse_item(":thing").unwrap().id, "minecraft:thing");
        assert_eq!(key(&parse_item("bow+").unwrap_err()), "argument.id.invalid");
    }

    fn coordinates(text: &str) -> Result<Coordinates, CommandError> {
        match ArgKind::Vec3.parse(&mut StringReader::new(text), &[])? {
            Arg::Vec3(coordinates) => Ok(coordinates),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn world_coordinates_center_whole_numbers_and_add_relatives() {
        let pose = crate::server::Pose {
            feet: [10.25, 64.0, -3.5],
            yaw: 0.0,
            pitch: 0.0,
        };
        assert_eq!(
            coordinates("1 2 3").unwrap().position(pose),
            [1.5, 2.0, 3.5]
        );
        assert_eq!(
            coordinates("1.0 2 -3.25").unwrap().position(pose),
            [1.0, 2.0, -3.25]
        );
        assert_eq!(
            coordinates("~ ~10 ~-1").unwrap().position(pose),
            [10.25, 74.0, -4.5]
        );
        assert_eq!(
            coordinates("~1 70 4").unwrap().position(pose),
            [11.25, 70.0, 4.5]
        );
        let incomplete = coordinates("1 2").unwrap_err();
        assert_eq!(
            (key(&incomplete), incomplete.cursor),
            ("argument.pos3d.incomplete".into(), Some(0))
        );
        assert_eq!(
            key(&coordinates("~ ^ ~").unwrap_err()),
            "argument.pos.mixed"
        );
        assert_eq!(
            key(&coordinates("^ ~ ^").unwrap_err()),
            "argument.pos.mixed"
        );
        assert_eq!(
            key(&coordinates("1 x 3").unwrap_err()),
            "parsing.double.expected"
        );
    }

    #[test]
    fn local_coordinates_turn_with_the_view() {
        let close = |a: [f64; 3], b: [f64; 3]| a.iter().zip(b).all(|(a, b)| (a - b).abs() < 1e-5);
        let south = crate::server::Pose {
            feet: [0.0, 64.0, 0.0],
            yaw: 0.0,
            pitch: 0.0,
        };
        let local = coordinates("^1 ^2 ^5").unwrap();
        assert!(
            close(local.position(south), [1.0, 66.0, 5.0]),
            "{:?}",
            local.position(south)
        );
        let west = crate::server::Pose { yaw: 90.0, ..south };
        assert!(
            close(local.position(west), [-5.0, 66.0, 1.0]),
            "{:?}",
            local.position(west)
        );
        let down = crate::server::Pose {
            pitch: 90.0,
            ..south
        };
        assert!(close(
            coordinates("^ ^ ^3").unwrap().position(down),
            [0.0, 61.0, 0.0]
        ));
    }

    #[test]
    fn block_positions_read_integers_or_relative_offsets() {
        let parse = |text: &str| match ArgKind::BlockPos.parse(&mut StringReader::new(text), &[]) {
            Ok(Arg::BlockPos(coordinates)) => Ok(coordinates),
            Ok(other) => panic!("{other:?}"),
            Err(error) => Err(error),
        };
        let pose = crate::server::Pose {
            feet: [10.7, 64.0, -3.2],
            yaw: 0.0,
            pitch: 0.0,
        };
        assert_eq!(parse("1 2 3").unwrap().position(pose), [1.0, 2.0, 3.0]);
        assert_eq!(parse("~ ~-1 ~0.5").unwrap().position(pose), [10.7, 63.0, -2.7]);
        assert_eq!(parse("-4 ~ 5").unwrap().position(pose), [-4.0, 64.0, 5.0]);
        assert_eq!(key(&parse("1.5 2 3").unwrap_err()), "parsing.int.invalid");
        let incomplete = parse("1 2").unwrap_err();
        assert_eq!(
            (key(&incomplete), incomplete.cursor),
            ("argument.pos3d.incomplete".into(), Some(0))
        );
        assert_eq!(key(&parse("").unwrap_err()), "argument.pos.missing.int");
        assert_eq!(key(&parse("1 ^ 3").unwrap_err()), "argument.pos.mixed");
        assert!(matches!(parse("^ ^1 ^"), Ok(Coordinates::Local(_))));
    }

    #[test]
    fn rotations_read_yaw_then_pitch() {
        let parse = |text: &str| ArgKind::Rotation.parse(&mut StringReader::new(text), &[]);
        let absolute = WorldCoordinate {
            value: 90.0,
            relative: false,
        };
        let relative = WorldCoordinate {
            value: -5.0,
            relative: true,
        };
        assert_eq!(parse("90 ~-5"), Ok(Arg::Rotation([absolute, relative])));
        let incomplete = parse("90").unwrap_err();
        assert_eq!(
            (key(&incomplete), incomplete.cursor),
            ("argument.rotation.incomplete".into(), Some(0))
        );
        assert_eq!(key(&parse("").unwrap_err()), "argument.rotation.incomplete");
        assert_eq!(key(&parse("^ 1").unwrap_err()), "argument.pos.mixed");
    }

    #[test]
    fn nbt_argument_spans_balanced_braces_with_strings() {
        let mut reader = StringReader::new(
            "{NoAI:1b,CustomName:\"a } b\",Tags:['x]',\"y\\\"}\"],Inner:{A:{}}} rest",
        );
        let Ok(Arg::Nbt(text)) = ArgKind::Nbt.parse(&mut reader, &[]) else {
            panic!("nbt");
        };
        assert_eq!(
            text,
            "{NoAI:1b,CustomName:\"a } b\",Tags:['x]',\"y\\\"}\"],Inner:{A:{}}}"
        );
        assert_eq!(reader.remaining(), " rest");
        let parse = |text: &str| ArgKind::Nbt.parse(&mut StringReader::new(text), &[]);
        assert_eq!(
            key(&parse("NoAI:1b").unwrap_err()),
            "argument.nbt.expected.compound"
        );
        assert_eq!(key(&parse("{NoAI:{}").unwrap_err()), "parsing.expected");
        assert_eq!(parse("{}"), Ok(Arg::Nbt("{}".into())));
    }

    #[test]
    fn decimal_arguments_report_bounds_like_brigadier() {
        let rate = ArgKind::Float {
            min: 1.0,
            max: 10000.0,
        };
        let mut reader = StringReader::at("rate 0.5", 5);
        let low = rate.parse(&mut reader, &[]).unwrap_err();
        assert_eq!(
            low.text,
            crate::text::Text::translate(
                "argument.float.low",
                vec![literal("1.0"), literal("0.5")]
            )
        );
        assert_eq!((low.cursor, reader.cursor), (Some(5), 5));
        let mut reader = StringReader::at("rate 20000", 5);
        let big = rate.parse(&mut reader, &[]).unwrap_err();
        assert_eq!(
            big.text,
            crate::text::Text::translate(
                "argument.float.big",
                vec![literal("10000.0"), literal("20000.0")]
            )
        );
        assert_eq!(big.cursor, Some(5));
        let mut reader = StringReader::new("40");
        assert_eq!(rate.parse(&mut reader, &[]), Ok(Arg::Float(40.0)));
        assert_eq!(
            rate.parse(&mut StringReader::new("1"), &[]),
            Ok(Arg::Float(1.0))
        );
        assert_eq!(
            key(&rate.parse(&mut StringReader::new("x"), &[]).unwrap_err()),
            "parsing.float.expected"
        );
    }
}
