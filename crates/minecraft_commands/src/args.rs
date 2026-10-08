use minecraftoss_player::GameMode;
use minecraftoss_world::game_rules::{self, Rule, Value};

use crate::reader::{CommandError, StringReader, literal};

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
pub enum ArgKind {
    Players { single: bool },
    Time { min: i32 },
    Rule,
    RuleValue,
    TimeMarker,
    GameMode,
    Greedy,
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
        }
    }
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
}
