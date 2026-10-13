use std::ops::Range;

use minecraftoss_world::game_rules::{RULES, Value};
use serde::{Deserialize, Serialize};

use crate::args::{Arg, ArgKind, ENCHANTMENT_COMPONENTS, Registry, allowed_in_id, resource_id};
use crate::permissions::GAMEMASTERS;
use crate::players::GAME_MODES;
use crate::reader::{CommandError, StringReader, literal};
use crate::server::CommandServer;
use crate::text::{AQUA, GOLD, GRAY, GREEN, LIGHT_PURPLE, RED, Text, YELLOW};
use crate::tree::{Node, NodeKind, Parsed, parse_from, smart_usage};

pub const SELECTORS: [&str; 4] = ["@p", "@a", "@r", "@s"];
pub const COMMAND_TIME_MARKERS: [&str; 4] = [
    "minecraft:day",
    "minecraft:midnight",
    "minecraft:night",
    "minecraft:noon",
];
const SELECTOR_KINDS: [char; 6] = ['p', 'a', 'r', 's', 'e', 'n'];
const TIME_UNITS: [&str; 4] = ["d", "s", "t", ""];
const ARGUMENT_COLORS: [u32; 5] = [AQUA, YELLOW, GREEN, LIGHT_PURPLE, GOLD];
const CONTEXT_AMOUNT: usize = 10;
const CONTEXT_HERE: &str = "<--[HERE]";

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CommandHints {
    pub players: Vec<String>,
    pub items: Vec<String>,
    pub entities: Vec<String>,
    pub effects: Vec<String>,
    pub enchantments: Vec<String>,
    pub hit: Option<[f64; 3]>,
    #[serde(default)]
    pub item_tags: Vec<String>,
    #[serde(default)]
    pub blocks: Vec<String>,
    #[serde(default)]
    pub block_tags: Vec<String>,
    #[serde(default)]
    pub hit_block: Option<[i32; 3]>,
    #[serde(default)]
    pub structures: Vec<String>,
    #[serde(default)]
    pub structure_tags: Vec<String>,
    #[serde(default)]
    pub biomes: Vec<String>,
    #[serde(default)]
    pub biome_tags: Vec<String>,
    #[serde(default)]
    pub poi_types: Vec<String>,
    #[serde(default)]
    pub poi_tags: Vec<String>,
    #[serde(default)]
    pub dimensions: Vec<String>,
    #[serde(default)]
    pub blocks_with_properties: Vec<String>,
}

impl CommandHints {
    pub fn from_server(server: &CommandServer) -> Self {
        let registries = server.registries();
        Self {
            players: server.player_names(),
            items: registries.items.clone(),
            entities: registries.entities.clone(),
            effects: registries.effects.clone(),
            enchantments: registries.enchantments.clone(),
            hit: None,
            item_tags: server.data().item_tags.keys().cloned().collect(),
            blocks: server.data().blocks.keys().cloned().collect(),
            block_tags: server.data().block_tags.keys().cloned().collect(),
            hit_block: None,
            structures: server.data().structures.clone(),
            structure_tags: server.data().structure_tags.keys().cloned().collect(),
            biomes: server.data().biomes.clone(),
            biome_tags: server.data().biome_tags.keys().cloned().collect(),
            poi_types: server.data().poi_types.clone(),
            poi_tags: server.data().poi_tags.keys().cloned().collect(),
            dimensions: vec![crate::server::OVERWORLD.to_owned()],
            blocks_with_properties: server
                .data()
                .blocks
                .iter()
                .filter(|(_, properties)| !properties.is_empty())
                .map(|(id, _)| id.clone())
                .collect(),
        }
    }

    pub fn ids(&self, registry: Registry) -> &[String] {
        match registry {
            Registry::EntityType => &self.entities,
            Registry::MobEffect => &self.effects,
            Registry::Enchantment => &self.enchantments,
            Registry::Structure => &self.structures,
            Registry::Biome => &self.biomes,
            Registry::PointOfInterest => &self.poi_types,
            Registry::Dimension => &self.dimensions,
        }
    }

    pub fn tag_ids(&self, registry: Registry) -> &[String] {
        match registry {
            Registry::Structure => &self.structure_tags,
            Registry::Biome => &self.biome_tags,
            Registry::PointOfInterest => &self.poi_tags,
            Registry::EntityType
            | Registry::MobEffect
            | Registry::Enchantment
            | Registry::Dimension => &[],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Suggestion {
    pub range: Range<usize>,
    pub text: String,
}

impl Suggestion {
    pub fn apply(&self, input: &str) -> String {
        let head = input.get(..self.range.start).unwrap_or(input);
        let tail = input.get(self.range.end..).unwrap_or("");
        format!("{head}{}{tail}", self.text)
    }

    pub fn cursor(&self) -> usize {
        self.range.start + self.text.len()
    }

    fn expand(&self, input: &str, range: &Range<usize>) -> Self {
        let before = input.get(range.start..self.range.start).unwrap_or("");
        let after = input.get(self.range.end..range.end).unwrap_or("");
        Self {
            range: range.clone(),
            text: format!("{before}{}{after}", self.text),
        }
    }
}

pub fn suggestion_suffix<'a>(input: &str, applied: &'a str) -> Option<&'a str> {
    applied.strip_prefix(input)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Completion {
    pub command: bool,
    pub start: usize,
    pub list: Vec<Suggestion>,
    pub usage: Vec<Text>,
    pub usage_start: usize,
    pub spans: Vec<(Range<usize>, u32)>,
}

pub fn matches_sub_str(typed: &str, candidate: &str) -> bool {
    let mut from = 0;
    loop {
        let rest = &candidate[from..];
        if rest.starts_with(typed) {
            return true;
        }
        match rest.find(['.', '_', '/']) {
            Some(at) => from += at + 1,
            None => return false,
        }
    }
}

pub fn last_word_index(text: &str) -> usize {
    text.char_indices()
        .rev()
        .find(|(_, c)| matches!(c, ' ' | '\t' | '\n' | '\u{b}' | '\u{c}' | '\r'))
        .map_or(0, |(at, c)| at + c.len_utf8())
}

pub fn complete(input: &str, cursor: usize, level: u8, hints: &CommandHints) -> Completion {
    complete_in(crate::commands::tree(), input, cursor, level, hints)
}

fn complete_in(
    roots: &[Node],
    input: &str,
    cursor: usize,
    level: u8,
    hints: &CommandHints,
) -> Completion {
    let mut cursor = cursor.min(input.len());
    while !input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    if !input.starts_with('/') {
        return chat(input, cursor, hints);
    }
    let parsed = parse_from(roots, input, 1, level);
    let spans = colors(&parsed, input);
    if cursor < 1 {
        return Completion {
            command: true,
            spans,
            ..Completion::default()
        };
    }
    let (parent, start) = suggestion_context(&parsed, cursor);
    let children = parent.map_or(roots, |node| node.children.as_slice());
    let mut found = Found::new(&input[..cursor]);
    for child in children.iter().filter(|child| child.visible(level)) {
        match child.kind {
            NodeKind::Literal(name) => found.literal(start, name),
            NodeKind::Argument(_, kind) => match child.suggestions {
                Some(list) => found.matching(start, list.iter().copied()),
                None => suggest_argument(kind, start, &parsed, level, hints, &mut found),
            },
        }
    }
    let list = found.finish();
    let optional = parent.is_some_and(|node| node.exec.is_some());
    let nodes = node_usage(children, optional, level);
    let usage = usage(&parsed, input, cursor, list.is_empty(), nodes);
    Completion {
        command: true,
        start: list
            .first()
            .map_or(start, |suggestion| suggestion.range.start),
        list,
        usage,
        usage_start: start,
        spans,
    }
}

fn chat(input: &str, cursor: usize, hints: &CommandHints) -> Completion {
    if input.trim().is_empty() {
        return Completion::default();
    }
    let typed = &input[..cursor];
    let start = last_word_index(typed);
    let mut found = Found::new(typed);
    found.matching(start, hints.players.iter().map(String::as_str));
    Completion {
        start,
        list: found.finish(),
        ..Completion::default()
    }
}

struct Found<'a> {
    input: &'a str,
    list: Vec<Suggestion>,
}

impl<'a> Found<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            input,
            list: Vec::new(),
        }
    }

    fn typed(&self, start: usize) -> &'a str {
        self.input.get(start..).unwrap_or("")
    }

    fn add(&mut self, start: usize, text: impl Into<String>) {
        let text = text.into();
        if start <= self.input.len() && text != self.typed(start) {
            self.list.push(Suggestion {
                range: start..self.input.len(),
                text,
            });
        }
    }

    fn literal(&mut self, start: usize, name: &str) {
        if name
            .to_lowercase()
            .starts_with(&self.typed(start).to_lowercase())
        {
            self.add(start, name);
        }
    }

    fn matching<'s>(&mut self, start: usize, candidates: impl IntoIterator<Item = &'s str>) {
        let typed = self.typed(start).to_lowercase();
        for candidate in candidates {
            if matches_sub_str(&typed, &candidate.to_lowercase()) {
                self.add(start, candidate);
            }
        }
    }

    fn resources<'s>(
        &mut self,
        start: usize,
        ids: impl IntoIterator<Item = &'s str>,
        suffix: &str,
    ) {
        let typed = self.typed(start).to_lowercase();
        for id in ids {
            if let Some(full) = resource_match(&typed, id) {
                self.add(start, format!("{full}{suffix}"));
            }
        }
    }

    fn tags<'s>(&mut self, start: usize, ids: impl IntoIterator<Item = &'s str>) {
        let typed = self.typed(start).to_lowercase();
        let Some(rest) = typed.strip_prefix('#') else {
            return;
        };
        for id in ids {
            if let Some(full) = resource_match(rest, id) {
                self.add(start, format!("#{full}"));
            }
        }
    }

    fn finish(self) -> Vec<Suggestion> {
        client_order(merge(self.input, self.list), self.input)
    }
}

fn resource_match(typed: &str, id: &str) -> Option<String> {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let full = format!("{namespace}:{path}");
    let hit = if typed.contains(':') {
        matches_sub_str(typed, &full)
    } else {
        matches_sub_str(typed, namespace) || matches_sub_str(typed, path)
    };
    hit.then_some(full)
}

fn merge(input: &str, found: Vec<Suggestion>) -> Vec<Suggestion> {
    let Some(start) = found.iter().map(|suggestion| suggestion.range.start).min() else {
        return Vec::new();
    };
    let end = found
        .iter()
        .map(|suggestion| suggestion.range.end)
        .max()
        .unwrap_or(start);
    let range = start..end;
    let mut list: Vec<Suggestion> = found
        .iter()
        .map(|suggestion| suggestion.expand(input, &range))
        .collect();
    list.sort_by_cached_key(|suggestion| (suggestion.text.to_lowercase(), suggestion.text.clone()));
    list.dedup();
    list
}

fn client_order(list: Vec<Suggestion>, input: &str) -> Vec<Suggestion> {
    let word = input[last_word_index(input)..].to_lowercase();
    let namespaced = format!("minecraft:{word}");
    let (mut first, rest): (Vec<Suggestion>, Vec<Suggestion>) =
        list.into_iter().partition(|suggestion| {
            suggestion.text.starts_with(&word) || suggestion.text.starts_with(&namespaced)
        });
    first.extend(rest);
    first
}

fn suggestion_context<'t>(parsed: &Parsed<'t>, cursor: usize) -> (Option<&'t Node>, usize) {
    for (index, stage) in parsed.stages.iter().enumerate() {
        let nodes = parsed.stage_nodes(index);
        let ranges = &parsed.ranges[nodes.clone()];
        let end = ranges.last().map_or(stage.start, |range| range.end);
        if end < cursor {
            if index + 1 < parsed.stages.len() {
                continue;
            }
            return match ranges.last() {
                Some(range) => (parsed.nodes.last().copied(), range.end + 1),
                None => (stage.root, stage.start),
            };
        }
        let mut parent = stage.root;
        for (node, range) in parsed.nodes[nodes].iter().zip(ranges) {
            if range.start <= cursor && cursor <= range.end {
                return (parent, range.start);
            }
            parent = Some(*node);
        }
        return (parent, stage.start);
    }
    (None, 1)
}

fn suggest_argument(
    kind: ArgKind,
    start: usize,
    parsed: &Parsed,
    level: u8,
    hints: &CommandHints,
    found: &mut Found,
) {
    match kind {
        ArgKind::Players { .. } => players(start, level, hints, found),
        ArgKind::Time { .. } => time_units(start, found),
        ArgKind::Rule => {
            for rule in &RULES {
                found.literal(start, rule.id);
                found.literal(start, &rule.name());
            }
        }
        ArgKind::RuleValue => {
            let boolean = parsed.args.iter().rev().find_map(|(_, arg)| match arg {
                Arg::Rule(rule) => Some(matches!(rule.default, Value::Bool(_))),
                _ => None,
            });
            if boolean == Some(true) {
                booleans(start, found);
            }
        }
        ArgKind::TimeMarker => found.resources(start, COMMAND_TIME_MARKERS, ""),
        ArgKind::GameMode => found.matching(start, GAME_MODES.map(|mode| mode.name())),
        ArgKind::Bool => booleans(start, found),
        ArgKind::Item => item(start, hints, found),
        ArgKind::Vec3 => coordinates(start, hints, found),
        ArgKind::Resource(registry) => {
            found.resources(start, hints.ids(registry).iter().map(String::as_str), "")
        }
        ArgKind::ItemPredicate => item_predicate(start, hints, found),
        ArgKind::ResourceOrTag(registry) => {
            let typed = found.typed(start);
            let tags = hints.tag_ids(registry).iter().map(String::as_str);
            if typed.starts_with('#') {
                found.tags(start, tags);
            } else {
                if typed.is_empty() {
                    for tag in tags {
                        found.add(start, format!("#{tag}"));
                    }
                }
                found.resources(start, hints.ids(registry).iter().map(String::as_str), "");
            }
        }
        ArgKind::BlockPos => block_coordinates(start, hints, found),
        ArgKind::BlockState => block_state(start, hints, found),
        ArgKind::BlockPredicate => {
            let typed = found.typed(start);
            let tags = hints.block_tags.iter().map(String::as_str);
            if typed.starts_with('#') {
                found.tags(start, tags);
            } else {
                if typed.is_empty() {
                    for tag in tags {
                        found.add(start, format!("#{tag}"));
                    }
                }
                block_state(start, hints, found);
            }
        }
        ArgKind::Greedy
        | ArgKind::Word
        | ArgKind::Int { .. }
        | ArgKind::Nbt
        | ArgKind::Rotation
        | ArgKind::Float { .. } => {}
    }
}

fn players(start: usize, level: u8, hints: &CommandHints, found: &mut Found) {
    let typed = found.typed(start);
    let selectors = level >= GAMEMASTERS;
    if let Some(kind) = typed.strip_prefix('@') {
        let known = kind
            .chars()
            .next()
            .is_some_and(|c| SELECTOR_KINDS.contains(&c));
        if selectors && !known {
            for selector in SELECTORS {
                found.add(start, selector);
            }
        }
        return;
    }
    found.matching(start, hints.players.iter().map(String::as_str));
    if selectors && typed.is_empty() {
        for selector in SELECTORS {
            found.add(start, selector);
        }
    }
}

fn time_units(start: usize, found: &mut Found) {
    let mut reader = StringReader::new(found.typed(start));
    if reader.read_float().is_ok() {
        found.matching(start + reader.cursor, TIME_UNITS);
    }
}

fn booleans(start: usize, found: &mut Found) {
    for value in ["true", "false"] {
        found.literal(start, value);
    }
}

fn id_length(text: &str) -> usize {
    text.find(|c: char| !allowed_in_id(c)).unwrap_or(text.len())
}

fn skip_whitespace(input: &str, at: usize) -> usize {
    let mut reader = StringReader::at(input, at);
    reader.skip_ws();
    reader.cursor
}

fn item(start: usize, hints: &CommandHints, found: &mut Found) {
    let typed = found.typed(start);
    let length = id_length(typed);
    let known = resource_id(&mut StringReader::new(&typed[..length]))
        .is_ok_and(|id| hints.items.contains(&id));
    if !known {
        found.resources(start, hints.items.iter().map(String::as_str), "");
        return;
    }
    let after = start + length;
    match typed[length..].chars().next() {
        None => found.add(after, "["),
        Some('[') => components(after + 1, found),
        Some('+') => kit_enchantments(after, hints, found),
        Some(_) => {}
    }
}

fn item_predicate(start: usize, hints: &CommandHints, found: &mut Found) {
    let typed = found.typed(start);
    let tags = hints.item_tags.iter().map(String::as_str);
    if typed.starts_with('#') {
        found.tags(start, tags);
        return;
    }
    if typed.is_empty() {
        for tag in tags {
            found.add(start, format!("#{tag}"));
        }
    }
    found.resources(start, hints.items.iter().map(String::as_str), "");
}

fn block_state(start: usize, hints: &CommandHints, found: &mut Found) {
    let typed = found.typed(start);
    let length = id_length(typed);
    let id = resource_id(&mut StringReader::new(&typed[..length])).ok();
    let known = |list: &[String]| id.as_ref().is_some_and(|id| list.contains(id));
    if !known(&hints.blocks) {
        found.resources(start, hints.blocks.iter().map(String::as_str), "");
    } else if length == typed.len() && known(&hints.blocks_with_properties) {
        found.add(start + length, "[");
    }
}

fn components(mut at: usize, found: &mut Found) {
    let input = found.input;
    while !input[at..].starts_with(']') {
        at = skip_whitespace(input, at);
        let key_end = at + id_length(&input[at..]);
        let key = resource_id(&mut StringReader::new(&input[at..key_end]));
        if !key.is_ok_and(|key| ENCHANTMENT_COMPONENTS.contains(&key.as_str())) {
            found.resources(at, ENCHANTMENT_COMPONENTS, "=");
            return;
        }
        at = skip_whitespace(input, key_end);
        if at == input.len() {
            found.add(at, "=");
            return;
        }
        if !input[at..].starts_with('=') {
            return;
        }
        let mut reader = StringReader::at(input, skip_whitespace(input, at + 1));
        if ArgKind::Nbt.parse(&mut reader, &[]).is_err() {
            return;
        }
        at = skip_whitespace(input, reader.cursor);
        if at == input.len() {
            found.add(at, ",");
            found.add(at, "]");
            return;
        }
        if !input[at..].starts_with(',') {
            return;
        }
        at = skip_whitespace(input, at + 1);
    }
}

fn kit_enchantments(at: usize, hints: &CommandHints, found: &mut Found) {
    let typed = found.typed(at);
    let Some(plus) = typed.rfind('+') else {
        return;
    };
    let id = &typed[plus + 1..];
    if id_length(id) == id.len() {
        let ids = hints.enchantments.iter().map(String::as_str);
        found.resources(at + plus + 1, ids, "");
    }
}

fn two_decimals(value: f64) -> String {
    format!("{:.2}", (value * 100.0).round() / 100.0)
}

fn coordinates(start: usize, hints: &CommandHints, found: &mut Found) {
    let hit = hints.hit.map(|hit| hit.map(two_decimals));
    suggest_coordinates(start, hit, ArgKind::Vec3, found);
}

fn block_coordinates(start: usize, hints: &CommandHints, found: &mut Found) {
    let hit = hints.hit_block.map(|hit| hit.map(|axis| axis.to_string()));
    suggest_coordinates(start, hit, ArgKind::BlockPos, found);
}

fn suggest_coordinates(start: usize, hit: Option<[String; 3]>, kind: ArgKind, found: &mut Found) {
    let typed = found.typed(start);
    let [x, y, z] = if typed.starts_with('^') {
        ["^", "^", "^"].map(str::to_owned)
    } else {
        hit.unwrap_or_else(|| ["~", "~", "~"].map(str::to_owned))
    };
    let mut parts: Vec<&str> = typed.split(' ').collect();
    while parts.last() == Some(&"") {
        parts.pop();
    }
    let candidates = match (typed.is_empty(), parts.as_slice()) {
        (true, _) => vec![x.clone(), format!("{x} {y}"), format!("{x} {y} {z}")],
        (false, [first]) => vec![format!("{first} {y}"), format!("{first} {y} {z}")],
        (false, [first, second]) => vec![format!("{first} {second} {z}")],
        _ => Vec::new(),
    };
    let valid = candidates
        .last()
        .is_some_and(|full| kind.parse(&mut StringReader::new(full), &[]).is_ok());
    if valid {
        found.matching(start, candidates.iter().map(String::as_str));
    }
}

fn node_usage(children: &[Node], optional: bool, level: u8) -> Vec<Text> {
    children
        .iter()
        .filter(|child| !child.is_literal())
        .filter_map(|child| smart_usage(child, level, optional, false))
        .map(|usage| Text::literal(usage).color(GRAY))
        .collect()
}

fn usage(parsed: &Parsed, input: &str, cursor: usize, empty: bool, nodes: Vec<Text>) -> Vec<Text> {
    let mut lines = Vec::new();
    let mut unparsed = false;
    if cursor == input.len() {
        if empty && !parsed.errors.is_empty() {
            lines.extend(
                parsed
                    .errors
                    .iter()
                    .map(|error| error_message(error, input)),
            );
        } else {
            unparsed = parsed.cursor < input.len();
        }
    }
    if lines.is_empty() {
        if nodes.is_empty() && unparsed {
            lines.push(error_message(&parse_error(parsed), input));
        }
        lines.extend(nodes);
    }
    lines
}

fn parse_error(parsed: &Parsed) -> CommandError {
    if let [only] = parsed.errors.as_slice() {
        return only.clone();
    }
    let key = if parsed.nodes.is_empty() {
        "command.unknown.command"
    } else {
        "command.unknown.argument"
    };
    CommandError::new(key, Vec::new()).at(parsed.cursor)
}

fn error_message(error: &CommandError, input: &str) -> Text {
    let Some(cursor) = error.cursor else {
        return error.text.clone();
    };
    let mut cursor = cursor.min(input.len());
    while !input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let head = &input[..cursor];
    let count = head.chars().count();
    let mut context = String::new();
    if count > CONTEXT_AMOUNT {
        context.push_str("...");
    }
    context.extend(head.chars().skip(count.saturating_sub(CONTEXT_AMOUNT)));
    context.push_str(CONTEXT_HERE);
    Text::translate(
        "command.context.parse_error",
        vec![error.text.clone(), literal(count), Text::literal(context)],
    )
}

fn span(spans: &mut Vec<(Range<usize>, u32)>, range: Range<usize>, color: u32) {
    if range.start < range.end {
        spans.push((range, color));
    }
}

fn colors(parsed: &Parsed, input: &str) -> Vec<(Range<usize>, u32)> {
    let mut spans = Vec::new();
    let mut done = 0;
    let last = parsed.stage_nodes(parsed.stages.len() - 1);
    let arguments = parsed.nodes[last.clone()]
        .iter()
        .zip(&parsed.ranges[last])
        .filter(|(node, _)| !node.is_literal());
    for (index, (_, range)) in arguments.enumerate() {
        if range.start >= input.len() {
            break;
        }
        let end = range.end.min(input.len());
        let color = ARGUMENT_COLORS[index % ARGUMENT_COLORS.len()];
        span(&mut spans, done..range.start, GRAY);
        span(&mut spans, range.start..end, color);
        done = end;
    }
    if parsed.cursor < input.len() {
        span(&mut spans, done..parsed.cursor, GRAY);
        span(&mut spans, parsed.cursor..input.len(), RED);
        done = input.len();
    }
    span(&mut spans, done..input.len(), GRAY);
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::args::TIME_MARKERS;
    use crate::permissions::{ALL, OWNERS};
    use crate::protocol::PlayerId;
    use crate::text::WHITE;

    fn owned(ids: &[&str]) -> Vec<String> {
        ids.iter().map(|id| (*id).to_owned()).collect()
    }

    fn hints() -> CommandHints {
        CommandHints {
            players: owned(&["Steve", "Alex"]),
            items: owned(&[
                "minecraft:stone",
                "minecraft:diamond_sword",
                "minecraft:diamond_pickaxe",
                "minecraft:iron_pickaxe",
                "minecraft:stick",
            ]),
            entities: owned(&["minecraft:cow", "minecraft:zombie"]),
            effects: owned(&["minecraft:regeneration", "minecraft:speed"]),
            enchantments: owned(&["minecraft:efficiency", "minecraft:sharpness"]),
            hit: None,
            item_tags: owned(&["minecraft:logs", "minecraft:planks"]),
            blocks: owned(&["minecraft:stone", "minecraft:oak_stairs", "minecraft:glass"]),
            block_tags: owned(&["minecraft:logs", "minecraft:stairs"]),
            hit_block: None,
            structures: owned(&["minecraft:village_plains", "minecraft:igloo"]),
            structure_tags: owned(&["minecraft:village"]),
            biomes: owned(&["minecraft:cherry_grove", "minecraft:desert"]),
            dimensions: owned(&["minecraft:overworld"]),
            blocks_with_properties: owned(&["minecraft:oak_stairs"]),
            ..CommandHints::default()
        }
    }

    fn at_end(input: &str, level: u8) -> Completion {
        complete(input, input.len(), level, &hints())
    }

    fn texts(completion: &Completion) -> Vec<&str> {
        completion
            .list
            .iter()
            .map(|suggestion| suggestion.text.as_str())
            .collect()
    }

    fn lang(key: &str) -> Option<String> {
        let text = match key {
            "command.context.parse_error" => "%s at position %s: %s",
            "argument.entity.selector.not_allowed" => "Selector not allowed",
            _ => return crate::server::tests::lang(key),
        };
        Some(text.to_owned())
    }

    fn usage_lines(completion: &Completion) -> Vec<(String, u32)> {
        completion
            .usage
            .iter()
            .map(|line| {
                let resolved = line.resolve(&lang);
                (resolved.text, resolved.color)
            })
            .collect()
    }

    #[test]
    fn root_literals_are_filtered_by_level() {
        let friend = at_end("/", ALL);
        assert_eq!(texts(&friend), vec!["help", "msg", "seed", "tell", "w"]);
        assert_eq!((friend.start, friend.command), (1, true));
        assert_eq!(friend.list[0].range, 1..1);
        assert_eq!(
            texts(&at_end("/", OWNERS)),
            vec![
                "clear",
                "deop",
                "difficulty",
                "effect",
                "enchant",
                "execute",
                "experience",
                "fill",
                "gamemode",
                "gamerule",
                "give",
                "help",
                "kill",
                "kit",
                "locate",
                "msg",
                "op",
                "seed",
                "setblock",
                "spawnpoint",
                "summon",
                "tag",
                "teleport",
                "tell",
                "tick",
                "time",
                "tp",
                "w",
                "weather",
                "xp",
            ]
        );
        let ga = at_end("/ga", OWNERS);
        assert_eq!(texts(&ga), vec!["gamemode", "gamerule"]);
        assert_eq!(ga.list[1].range, 1..3);
        assert!(at_end("/ga", ALL).list.is_empty());
        assert!(at_end("/seed", ALL).list.is_empty());
        assert!(complete("/ga", 0, OWNERS, &hints()).list.is_empty());
    }

    #[test]
    fn game_modes_complete_from_the_argument_start() {
        let typed = at_end("/gamemode c", OWNERS);
        assert_eq!(texts(&typed), vec!["creative"]);
        assert_eq!((typed.start, typed.list[0].range.clone()), (10, 10..11));
        assert_eq!(
            texts(&at_end("/gamemode ", OWNERS)),
            vec!["adventure", "creative", "spectator", "survival"]
        );
        assert!(at_end("/gamemode creative", OWNERS).list.is_empty());
        let target = at_end("/gamemode creative ", OWNERS);
        assert_eq!(
            texts(&target),
            vec!["@a", "@p", "@r", "@s", "Alex", "Steve"]
        );
        assert_eq!(target.start, 19);
    }

    #[test]
    fn items_complete_by_path_words_and_offer_components_once_known() {
        let pick = at_end("/give @s pick", OWNERS);
        assert_eq!(
            texts(&pick),
            vec!["minecraft:diamond_pickaxe", "minecraft:iron_pickaxe"]
        );
        assert_eq!((pick.start, pick.list[0].range.clone()), (9, 9..13));
        assert_eq!(
            texts(&at_end("/give @s s", OWNERS)),
            vec![
                "minecraft:stick",
                "minecraft:stone",
                "minecraft:diamond_sword"
            ]
        );
        let stone = at_end("/give @s stone", OWNERS);
        assert_eq!(texts(&stone), vec!["["]);
        assert_eq!(stone.list[0].range, 14..14);
        assert_eq!(
            texts(&at_end("/give @s Stone", OWNERS)),
            vec!["minecraft:stone"]
        );
    }

    #[test]
    fn item_components_suggest_keys_assignment_and_close_like_vanilla() {
        let keys = vec!["minecraft:enchantments=", "minecraft:stored_enchantments="];
        assert_eq!(texts(&at_end("/give @s diamond_sword[", OWNERS)), keys);
        let partial = at_end("/give @s diamond_sword[ench", OWNERS);
        assert_eq!(texts(&partial), keys);
        assert_eq!(partial.list[0].range, 23..27);
        assert_eq!(
            texts(&at_end("/give @s diamond_sword[enchantments", OWNERS)),
            vec!["="]
        );
        assert_eq!(
            texts(&at_end(
                "/give @s diamond_sword[enchantments={sharpness:5}",
                OWNERS
            )),
            vec![",", "]"]
        );
        assert_eq!(
            texts(&at_end(
                "/give @s diamond_sword[enchantments={sharpness:5}, ",
                OWNERS
            )),
            keys
        );
        assert!(
            at_end("/give @s diamond_sword[enchantments={sharp", OWNERS)
                .list
                .is_empty()
        );
        assert!(at_end("/give @s diamond_sword[]", OWNERS).list.is_empty());
        assert!(at_end("/give @s notanitem[", OWNERS).list.is_empty());
    }

    #[test]
    fn item_predicates_suggest_items_and_hash_tags() {
        let empty = at_end("/clear @s ", OWNERS);
        assert_eq!(texts(&empty)[0], "#minecraft:logs");
        assert!(texts(&empty).contains(&"minecraft:stone"));
        let tag = at_end("/clear @s #lo", OWNERS);
        assert_eq!(texts(&tag), vec!["#minecraft:logs"]);
        assert_eq!(tag.list[0].range, 10..13);
        assert_eq!(
            texts(&at_end("/clear @s #minecraft:p", OWNERS)),
            vec!["#minecraft:planks"]
        );
        assert_eq!(
            texts(&at_end("/clear @s sto", OWNERS)),
            vec!["minecraft:stone"]
        );
        assert!(at_end("/clear @s stone ", OWNERS).list.is_empty());
    }

    #[test]
    fn kit_plus_syntax_suggests_enchantments() {
        let kit = at_end("/give @s diamond_pickaxe+eff", OWNERS);
        assert_eq!(texts(&kit), vec!["minecraft:efficiency"]);
        assert_eq!(kit.list[0].range, 25..28);
        assert_eq!(
            texts(&at_end("/give @s diamond_pickaxe+", OWNERS)),
            vec!["minecraft:efficiency", "minecraft:sharpness"]
        );
        assert!(
            at_end("/give @s diamond_pickaxe+efficiency=", OWNERS)
                .list
                .is_empty()
        );
    }

    #[test]
    fn players_complete_names_and_selectors() {
        let kill = at_end("/kill ", OWNERS);
        assert_eq!(texts(&kill), vec!["@a", "@p", "@r", "@s", "Alex", "Steve"]);
        assert_eq!(kill.start, 6);
        let at = at_end("/kill @", OWNERS);
        assert_eq!(texts(&at), vec!["@a", "@p", "@r", "@s"]);
        assert_eq!(at.list[0].range, 6..7);
        assert_eq!(
            texts(&at_end("/kill @x", OWNERS)),
            vec!["@a", "@p", "@r", "@s"]
        );
        assert!(at_end("/kill @s", OWNERS).list.is_empty());
        assert_eq!(texts(&at_end("/kill ST", OWNERS)), vec!["Steve"]);
    }

    #[test]
    fn usage_shows_when_nothing_matches_and_errors_read_like_vanilla() {
        let give = at_end("/give ", OWNERS);
        assert_eq!(
            usage_lines(&give),
            vec![("<targets> <item> [<count>]".to_owned(), GRAY)]
        );
        assert_eq!(give.usage_start, 6);
        assert_eq!(
            usage_lines(&at_end("/kill ", OWNERS)),
            vec![("[<targets>]".to_owned(), GRAY)]
        );
        assert_eq!(
            usage_lines(&at_end("/kill @e", OWNERS)),
            vec![(
                "Selector not allowed at position 6: /kill <--[HERE]".to_owned(),
                WHITE
            )]
        );
        assert_eq!(
            usage_lines(&at_end("/gamemode god", OWNERS)),
            vec![(
                "Unknown game mode: god at position 13: ...memode god<--[HERE]".to_owned(),
                WHITE
            )]
        );
        assert_eq!(
            usage_lines(&at_end("/frob", OWNERS)),
            vec![(
                "Unknown or incomplete command. See below for error at position 1: /<--[HERE]"
                    .to_owned(),
                WHITE
            )]
        );
        assert!(at_end("/seed", OWNERS).usage.is_empty());
        let middle = complete("/gamemode god", 3, OWNERS, &hints());
        assert_eq!(texts(&middle), vec!["gamemode", "gamerule"]);
        assert!(middle.usage.is_empty());
    }

    #[test]
    fn arguments_cycle_colours_and_unparsed_text_is_red() {
        assert_eq!(
            at_end("/gamemode creative Steve", OWNERS).spans,
            vec![
                (0..10, GRAY),
                (10..18, AQUA),
                (18..19, GRAY),
                (19..24, YELLOW)
            ]
        );
        assert_eq!(
            at_end("/gamemode god", OWNERS).spans,
            vec![(0..10, GRAY), (10..13, RED)]
        );
        assert_eq!(
            at_end("/give @s stone 1", OWNERS).spans,
            vec![
                (0..6, GRAY),
                (6..8, AQUA),
                (8..9, GRAY),
                (9..14, YELLOW),
                (14..15, GRAY),
                (15..16, GREEN)
            ]
        );
        assert!(complete("hello", 5, OWNERS, &hints()).spans.is_empty());
    }

    #[test]
    fn coordinates_suggest_tildes_or_the_point_in_view() {
        assert_eq!(
            texts(&at_end("/tp ", OWNERS)),
            vec!["@a", "@p", "@r", "@s", "Alex", "Steve", "~", "~ ~", "~ ~ ~"]
        );
        assert_eq!(texts(&at_end("/tp ~ ", OWNERS)), vec!["~ ~", "~ ~ ~"]);
        assert_eq!(texts(&at_end("/tp ^", OWNERS)), vec!["^ ^", "^ ^ ^"]);
        assert!(at_end("/tp 1 2 3", OWNERS).list.is_empty());
        assert!(at_end("/tp x", OWNERS).list.is_empty());
        let looking = CommandHints {
            hit: Some([0.125, 64.0, -3.5]),
            ..hints()
        };
        let summon = complete("/summon cow ", 12, OWNERS, &looking);
        assert_eq!(
            texts(&summon),
            vec!["0.13", "0.13 64.00", "0.13 64.00 -3.50"]
        );
    }

    #[test]
    fn redirects_suggest_and_colour_the_target_command() {
        let tp = at_end("/tp ", OWNERS);
        assert_eq!(texts(&tp), texts(&at_end("/teleport ", OWNERS)));
        assert_eq!(tp.start, 4);
        assert_eq!(
            usage_lines(&at_end("/tp 1", OWNERS)),
            usage_lines(&at_end("/teleport 1", OWNERS))
        );
        assert_eq!(
            at_end("/tp @s ~ ~ ~", OWNERS).spans,
            vec![(0..4, GRAY), (4..6, AQUA), (6..7, GRAY), (7..12, YELLOW)]
        );
        assert_eq!(texts(&at_end("/tp", OWNERS)), Vec::<&str>::new());
        assert_eq!(
            texts(&at_end("/t", OWNERS)),
            vec!["tag", "teleport", "tell", "tick", "time", "tp"]
        );
        assert_eq!(texts(&at_end("/xp ", OWNERS)), vec!["add", "query", "set"]);
        assert_eq!(
            texts(&at_end("/xp add @s 5 ", OWNERS)),
            vec!["levels", "points"]
        );
        assert_eq!(
            usage_lines(&at_end("/xp query ", OWNERS)),
            vec![("<target> (levels|points)".to_owned(), GRAY)]
        );
    }

    #[test]
    fn block_arguments_suggest_whole_coordinates_and_block_ids() {
        assert_eq!(
            texts(&at_end("/setblock ", OWNERS)),
            vec!["~", "~ ~", "~ ~ ~"]
        );
        let looking = CommandHints {
            hit: Some([0.125, 64.0, -3.5]),
            hit_block: Some([0, 63, -4]),
            ..hints()
        };
        assert_eq!(
            texts(&complete("/setblock ", 10, OWNERS, &looking)),
            vec!["0", "0 63", "0 63 -4"]
        );
        assert_eq!(
            texts(&at_end("/setblock ~ ~ ~ st", OWNERS)),
            vec!["minecraft:stone", "minecraft:oak_stairs"]
        );
        assert!(at_end("/setblock ~ ~ ~ glass", OWNERS).list.is_empty());
        let stairs = at_end("/setblock ~ ~ ~ oak_stairs", OWNERS);
        assert_eq!(texts(&stairs), vec!["["]);
        assert_eq!(stairs.list[0].range, 26..26);
        assert_eq!(
            texts(&at_end(
                "/fill ~ ~ ~ ~ ~ ~ glass replace oak_stairs",
                OWNERS
            )),
            vec!["["]
        );
        assert_eq!(
            texts(&at_end("/setblock ~ ~ ~ glass ", OWNERS)),
            vec!["destroy", "keep", "replace", "strict"]
        );
        assert_eq!(
            texts(&at_end("/fill ~ ~ ~ ~ ~ ~ glass replace #st", OWNERS)),
            vec!["#minecraft:stairs"]
        );
        assert_eq!(
            texts(&at_end("/fill ~ ~ ~ ~ ~ ~ glass replace ", OWNERS))[..2],
            ["#minecraft:logs", "#minecraft:stairs"]
        );
        assert_eq!(
            texts(&at_end("/fill ~ ~ ~ ~ ~ ~ glass replace stone ", OWNERS)),
            vec!["destroy", "hollow", "outline", "strict"]
        );
    }

    #[test]
    fn locate_suggests_ids_and_tags_of_its_registry() {
        assert_eq!(
            texts(&at_end("/locate ", OWNERS)),
            vec!["biome", "poi", "structure"]
        );
        assert_eq!(
            texts(&at_end("/locate structure ", OWNERS)),
            vec![
                "#minecraft:village",
                "minecraft:igloo",
                "minecraft:village_plains"
            ]
        );
        assert_eq!(
            texts(&at_end("/locate structure #v", OWNERS)),
            vec!["#minecraft:village"]
        );
        assert_eq!(
            texts(&at_end("/locate biome ch", OWNERS)),
            vec!["minecraft:cherry_grove"]
        );
        assert!(at_end("/locate poi ", OWNERS).list.is_empty());
    }

    #[test]
    fn execute_suggests_its_subcommands_dimensions_and_any_command_after_run() {
        assert_eq!(
            texts(&at_end("/execute ", OWNERS)),
            vec!["in", "positioned", "run"]
        );
        assert_eq!(
            texts(&at_end("/execute in ", OWNERS)),
            vec!["minecraft:overworld"]
        );
        assert_eq!(
            texts(&at_end("/execute in minecraft:overworld ", OWNERS)),
            vec!["in", "positioned", "run"]
        );
        let run = at_end("/execute in minecraft:overworld run ", OWNERS);
        assert_eq!(texts(&run), texts(&at_end("/", OWNERS)));
        assert_eq!(run.start, 36);
        assert_eq!(
            texts(&at_end("/execute run gamemode c", OWNERS)),
            vec!["creative"]
        );
        assert_eq!(
            at_end(
                "/execute in minecraft:overworld run gamemode creative",
                OWNERS
            )
            .spans,
            vec![(0..45, GRAY), (45..53, AQUA)]
        );
        assert_eq!(
            usage_lines(&at_end("/execute positioned ", OWNERS)),
            vec![("<pos> -> execute".to_owned(), GRAY)]
        );
    }

    #[test]
    fn time_suggests_units_after_a_number_and_command_markers() {
        let units = at_end("/time set 10", OWNERS);
        assert_eq!(texts(&units), vec!["d", "s", "t"]);
        assert_eq!(units.list[0].range, 12..12);
        assert_eq!(
            texts(&at_end("/time set n", OWNERS)),
            vec!["minecraft:night", "minecraft:noon"]
        );
        assert_eq!(
            texts(&at_end("/time set ", OWNERS)),
            COMMAND_TIME_MARKERS.to_vec()
        );
        assert!(at_end("/time set 1d", OWNERS).list.is_empty());
        assert_eq!(
            texts(&at_end("/weather rain 5", OWNERS)),
            vec!["d", "s", "t"]
        );
        for marker in COMMAND_TIME_MARKERS {
            assert!(
                TIME_MARKERS
                    .iter()
                    .any(|(name, _)| marker == format!("minecraft:{name}"))
            );
        }
    }

    #[test]
    fn game_rules_complete_short_and_namespaced_names_and_boolean_values() {
        assert_eq!(
            texts(&at_end("/gamerule keep", OWNERS)),
            vec!["keep_inventory"]
        );
        assert_eq!(
            texts(&at_end("/gamerule minecraft:keep", OWNERS)),
            vec!["minecraft:keep_inventory"]
        );
        assert_eq!(at_end("/gamerule ", OWNERS).list.len(), 2 * RULES.len());
        assert_eq!(
            texts(&at_end("/gamerule keep_inventory ", OWNERS)),
            vec!["false", "true"]
        );
        assert_eq!(
            texts(&at_end("/gamerule keep_inventory t", OWNERS)),
            vec!["true"]
        );
        assert!(
            at_end("/gamerule random_tick_speed ", OWNERS)
                .list
                .is_empty()
        );
    }

    #[test]
    fn registry_ids_match_namespace_or_path_words() {
        assert_eq!(
            texts(&at_end("/effect give @s rege", OWNERS)),
            vec!["minecraft:regeneration"]
        );
        assert_eq!(
            texts(&at_end("/summon z", OWNERS)),
            vec!["minecraft:zombie"]
        );
        assert_eq!(
            texts(&at_end("/summon minecraft:c", OWNERS)),
            vec!["minecraft:cow"]
        );
        assert_eq!(
            texts(&at_end("/summon m", OWNERS)),
            vec!["minecraft:cow", "minecraft:zombie"]
        );
        assert!(at_end("/summon x:c", OWNERS).list.is_empty());
    }

    #[test]
    fn chat_lines_complete_player_names_at_the_last_word() {
        let line = complete("hi st", 5, ALL, &hints());
        assert!(!line.command);
        assert_eq!((line.start, texts(&line)), (3, vec!["Steve"]));
        assert!(line.usage.is_empty());
        assert_eq!(
            texts(&complete("hi ", 3, ALL, &hints())),
            vec!["Alex", "Steve"]
        );
        assert_eq!(complete("   ", 3, ALL, &hints()), Completion::default());
    }

    #[test]
    fn suggestions_apply_over_their_range_and_show_the_rest_inline() {
        let creative = Suggestion {
            range: 10..11,
            text: "creative".to_owned(),
        };
        let applied = creative.apply("/gamemode c");
        assert_eq!(applied, "/gamemode creative");
        assert_eq!(creative.cursor(), 18);
        assert_eq!(suggestion_suffix("/gamemode c", &applied), Some("reative"));
        assert_eq!(suggestion_suffix("/gamemode x", &applied), None);
        let middle = complete("/gamemode creative", 3, OWNERS, &hints());
        assert_eq!(
            middle.list[0].apply("/gamemode creative"),
            "/gamemodememode creative"
        );
        assert_eq!(middle.list[0].cursor(), 9);
    }

    #[test]
    fn merged_suggestions_expand_to_one_range_without_duplicates() {
        let at = |range: Range<usize>, text: &str| Suggestion {
            range,
            text: text.to_owned(),
        };
        let merged = merge(
            "/time set 1",
            vec![at(11..11, "d"), at(10..11, "1x"), at(11..11, "d")],
        );
        assert_eq!(merged, vec![at(10..11, "1d"), at(10..11, "1x")]);
        assert!(merge("/", Vec::new()).is_empty());
    }

    #[test]
    fn matching_restarts_after_dots_underscores_and_slashes() {
        assert!(matches_sub_str("pick", "diamond_pickaxe"));
        assert!(!matches_sub_str("ax", "diamond_pickaxe"));
        assert!(matches_sub_str("", "anything"));
        assert!(matches_sub_str("gold", "minecraft:raw_gold"));
        assert!(matches_sub_str("dungeon", "chests/simple_dungeon"));
        assert!(matches_sub_str("bar", "foo.bar"));
        assert!(!matches_sub_str("craft", "minecraft"));
        assert_eq!(last_word_index("give @s  pi"), 9);
        assert_eq!(last_word_index("hello"), 0);
    }

    #[test]
    fn hints_come_from_the_server_players_and_registries() {
        let mut server = CommandServer::new(PlayerId(0));
        server.join(PlayerId(0), "Steve");
        server.join(PlayerId(1), "Alex");
        server.registries_mut().items = owned(&["minecraft:stone"]);
        server.registries_mut().enchantments = owned(&["minecraft:sharpness"]);
        let hints = CommandHints::from_server(&server);
        assert_eq!(hints.players, owned(&["Steve", "Alex"]));
        assert_eq!(hints.items, owned(&["minecraft:stone"]));
        assert_eq!(
            hints.ids(Registry::Enchantment),
            owned(&["minecraft:sharpness"]).as_slice()
        );
        assert_eq!(hints.hit, None);
        assert!(hints.item_tags.is_empty());
        server
            .data_mut()
            .item_tags
            .insert("minecraft:logs".into(), owned(&["minecraft:oak_log"]));
        server
            .data_mut()
            .blocks
            .insert("minecraft:stone".into(), Vec::new());
        server.data_mut().blocks.insert(
            "minecraft:oak_log".into(),
            vec![("axis".into(), owned(&["x", "y", "z"]))],
        );
        let hints = CommandHints::from_server(&server);
        assert_eq!(hints.item_tags, owned(&["minecraft:logs"]));
        assert_eq!(
            hints.blocks,
            owned(&["minecraft:oak_log", "minecraft:stone"])
        );
        assert_eq!(hints.blocks_with_properties, owned(&["minecraft:oak_log"]));
        let json = serde_json::to_string(&hints).unwrap();
        assert_eq!(serde_json::from_str::<CommandHints>(&json).unwrap(), hints);
    }

    #[test]
    fn fixed_suggestions_replace_the_argument_kinds_own() {
        use crate::tree::sorted;
        let rate = ArgKind::Float {
            min: 1.0,
            max: 10000.0,
        };
        let roots = sorted(vec![
            Node::literal("pace")
                .then(Node::argument("rate", rate).suggests(&["20"]))
                .then(Node::literal("plain").then(Node::argument("rate", rate)))
                .then(Node::literal("step").then(Node::literal("stop")).then(
                    Node::argument("time", ArgKind::Time { min: 1 }).suggests(&["1t", "1s"]),
                )),
        ]);
        let at_end = |input: &str| complete_in(&roots, input, input.len(), OWNERS, &hints());
        assert_eq!(texts(&at_end("/pace ")), vec!["20", "plain", "step"]);
        assert_eq!(texts(&at_end("/pace 2")), vec!["20"]);
        assert!(at_end("/pace plain ").list.is_empty());
        assert_eq!(texts(&at_end("/pace step ")), vec!["1s", "1t", "stop"]);
        let typed = at_end("/pace step 1");
        assert_eq!(texts(&typed), vec!["1s", "1t"]);
        assert_eq!(typed.list[0].range, 11..12);
    }
}
