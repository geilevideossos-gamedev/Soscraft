use minecraftoss_core::nbt::Tag;
use std::collections::BTreeMap;

pub const GAME_RULES: &str = "game_rules";
pub const KEEP_INVENTORY: &str = "minecraft:keep_inventory";
const NAMESPACE: &str = "minecraft:";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    Int(i32),
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool(value) => write!(f, "{value}"),
            Self::Int(value) => write!(f, "{value}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rule {
    pub id: &'static str,
    pub default: Value,
    pub min: i32,
    pub max: i32,
}

impl Rule {
    const fn on(id: &'static str) -> Self {
        Self::flag(id, true)
    }

    const fn off(id: &'static str) -> Self {
        Self::flag(id, false)
    }

    const fn flag(id: &'static str, default: bool) -> Self {
        Self {
            id,
            default: Value::Bool(default),
            min: 0,
            max: 0,
        }
    }

    const fn int(id: &'static str, default: i32, min: i32) -> Self {
        Self::bounded(id, default, min, i32::MAX)
    }

    const fn bounded(id: &'static str, default: i32, min: i32, max: i32) -> Self {
        Self {
            id,
            default: Value::Int(default),
            min,
            max,
        }
    }

    pub fn name(&self) -> String {
        format!("{NAMESPACE}{}", self.id)
    }

    pub fn parse(&self, text: &str) -> Result<Value, String> {
        match self.default {
            Value::Bool(_) => match text {
                "true" => Ok(Value::Bool(true)),
                "false" => Ok(Value::Bool(false)),
                _ => Err(format!(
                    "Invalid boolean, expected 'true' or 'false' but found '{text}'"
                )),
            },
            Value::Int(_) => {
                let value: i32 = text
                    .parse()
                    .map_err(|_| format!("Invalid integer '{text}'"))?;
                if value < self.min {
                    Err(format!(
                        "Integer must not be less than {}, found {value}",
                        self.min
                    ))
                } else if value > self.max {
                    Err(format!(
                        "Integer must not be more than {}, found {value}",
                        self.max
                    ))
                } else {
                    Ok(Value::Int(value))
                }
            }
        }
    }

    fn decode(&self, tag: &Tag) -> Option<Value> {
        let raw = tag.as_i64()?;
        Some(match self.default {
            Value::Bool(_) => Value::Bool(raw != 0),
            Value::Int(_) => Value::Int(raw.clamp(i64::from(self.min), i64::from(self.max)) as i32),
        })
    }

    fn encode(value: Value) -> Tag {
        match value {
            Value::Bool(value) => Tag::Byte(i8::from(value)),
            Value::Int(value) => Tag::Int(value),
        }
    }
}

pub const RULES: [Rule; 58] = [
    Rule::on("advance_time"),
    Rule::on("advance_weather"),
    Rule::on("allow_entering_nether_using_portals"),
    Rule::on("block_drops"),
    Rule::on("block_explosion_drop_decay"),
    Rule::on("command_blocks_work"),
    Rule::on("command_block_output"),
    Rule::on("drowning_damage"),
    Rule::on("elytra_movement_check"),
    Rule::on("ender_pearls_vanish_on_death"),
    Rule::on("entity_drops"),
    Rule::on("fall_damage"),
    Rule::on("fire_damage"),
    Rule::int("fire_spread_radius_around_player", 128, -1),
    Rule::on("forgive_dead_players"),
    Rule::on("freeze_damage"),
    Rule::on("global_sound_events"),
    Rule::off("immediate_respawn"),
    Rule::off("keep_inventory"),
    Rule::off("lava_source_conversion"),
    Rule::off("limited_crafting"),
    Rule::on("locator_bar"),
    Rule::on("log_admin_commands"),
    Rule::int("max_block_modifications", 32768, 1),
    Rule::int("max_command_forks", 65536, 0),
    Rule::int("max_command_sequence_length", 65536, 0),
    Rule::int("max_entity_cramming", 24, 0),
    Rule::bounded("max_snow_accumulation_height", 1, 0, 8),
    Rule::on("mob_drops"),
    Rule::on("mob_explosion_drop_decay"),
    Rule::on("mob_griefing"),
    Rule::on("natural_health_regeneration"),
    Rule::on("player_movement_check"),
    Rule::int("players_nether_portal_creative_delay", 0, 0),
    Rule::int("players_nether_portal_default_delay", 80, 0),
    Rule::int("players_sleeping_percentage", 100, 0),
    Rule::on("projectiles_can_break_blocks"),
    Rule::on("pvp"),
    Rule::on("raids"),
    Rule::int("random_tick_speed", 3, 0),
    Rule::off("reduced_debug_info"),
    Rule::int("respawn_radius", 10, 0),
    Rule::on("send_command_feedback"),
    Rule::on("show_advancement_messages"),
    Rule::on("show_death_messages"),
    Rule::on("spawner_blocks_work"),
    Rule::on("spawn_mobs"),
    Rule::on("spawn_monsters"),
    Rule::on("spawn_patrols"),
    Rule::on("spawn_phantoms"),
    Rule::on("spawn_wandering_traders"),
    Rule::on("spawn_wardens"),
    Rule::on("spectators_generate_chunks"),
    Rule::on("spread_vines"),
    Rule::on("tnt_explodes"),
    Rule::off("tnt_explosion_drop_decay"),
    Rule::off("universal_anger"),
    Rule::on("water_source_conversion"),
];

pub fn rule(id: &str) -> Option<&'static Rule> {
    let id = id.strip_prefix(NAMESPACE).unwrap_or(id);
    RULES.iter().find(|rule| rule.id == id)
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GameRules {
    values: BTreeMap<&'static str, Value>,
    unknown: BTreeMap<String, Tag>,
}

impl GameRules {
    pub fn get(&self, id: &str) -> Option<Value> {
        let rule = rule(id)?;
        Some(self.values.get(rule.id).copied().unwrap_or(rule.default))
    }

    pub fn bool(&self, id: &str) -> bool {
        matches!(self.get(id), Some(Value::Bool(true)))
    }

    pub fn int(&self, id: &str) -> i32 {
        match self.get(id) {
            Some(Value::Int(value)) => value,
            _ => 0,
        }
    }

    pub fn keep_inventory(&self) -> bool {
        self.bool(KEEP_INVENTORY)
    }

    pub fn set(&mut self, id: &str, text: &str) -> Result<Value, String> {
        let rule = rule(id).ok_or_else(|| format!("Unknown game rule '{id}'"))?;
        let value = rule.parse(text)?;
        self.put(rule, value);
        Ok(value)
    }

    fn put(&mut self, rule: &'static Rule, value: Value) {
        if value == rule.default {
            self.values.remove(rule.id);
        } else {
            self.values.insert(rule.id, value);
        }
    }

    pub fn from_tag(data: &Tag) -> Self {
        let mut rules = Self::default();
        for (key, tag) in data.as_compound().into_iter().flatten() {
            match rule(key) {
                Some(rule) => {
                    if let Some(value) = rule.decode(tag) {
                        rules.put(rule, value);
                    }
                }
                None => {
                    rules.unknown.insert(key.clone(), tag.clone());
                }
            }
        }
        rules
    }

    pub fn to_tag(&self) -> Tag {
        let mut data = self.unknown.clone();
        for rule in &RULES {
            let value = self.values.get(rule.id).copied().unwrap_or(rule.default);
            data.insert(rule.name(), Rule::encode(value));
        }
        Tag::Compound(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_follow_vanilla_26_3() {
        let rules = GameRules::default();
        assert!(!rules.keep_inventory());
        assert_eq!(rules.int("random_tick_speed"), 3);
        assert!(rules.bool("mob_griefing"));
        assert!(rules.bool("minecraft:advance_time"));
        assert_eq!(rules.int("max_snow_accumulation_height"), 1);
        assert_eq!(rules.int("fire_spread_radius_around_player"), 128);
        assert_eq!(rule("max_snow_accumulation_height").unwrap().max, 8);
        assert_eq!(rule("max_entity_cramming").unwrap().max, i32::MAX);
        assert_eq!(rule("max_minecart_speed"), None);
        assert_eq!(rules.get("keepInventory"), None);
        let mut ids: Vec<_> = RULES.iter().map(|rule| rule.id).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), RULES.len());
    }

    #[test]
    fn set_parses_bools_and_bounded_ints() {
        let mut rules = GameRules::default();
        assert_eq!(rules.set("keep_inventory", "true"), Ok(Value::Bool(true)));
        assert!(rules.keep_inventory());
        assert!(rules.set("keep_inventory", "yes").is_err());
        assert!(rules.keep_inventory());
        assert_eq!(
            rules.set("minecraft:keep_inventory", "false"),
            Ok(Value::Bool(false))
        );
        assert!(!rules.keep_inventory());
        assert_eq!(rules.set("random_tick_speed", "0"), Ok(Value::Int(0)));
        assert_eq!(rules.int("minecraft:random_tick_speed"), 0);
        assert!(rules.set("random_tick_speed", "-1").is_err());
        assert!(rules.set("random_tick_speed", "three").is_err());
        assert!(rules.set("max_snow_accumulation_height", "9").is_err());
        assert_eq!(
            rules.set("max_snow_accumulation_height", "8"),
            Ok(Value::Int(8))
        );
        assert!(rules.set("fire_spread_radius_around_player", "-1").is_ok());
        assert!(rules.set("no_such_rule", "true").is_err());
        assert!(rules.set("keepInventory", "true").is_err());
    }

    #[test]
    fn tags_use_namespaced_ids_bytes_and_ints() {
        let mut rules = GameRules::default();
        rules.set("keep_inventory", "true").unwrap();
        let tag = rules.to_tag();
        let entries = tag.as_compound().unwrap();
        assert_eq!(entries.len(), RULES.len());
        assert_eq!(tag.get("minecraft:keep_inventory"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("minecraft:mob_griefing"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("minecraft:pvp"), Some(&Tag::Byte(1)));
        assert_eq!(tag.get("minecraft:random_tick_speed"), Some(&Tag::Int(3)));
        assert_eq!(GameRules::from_tag(&tag), rules);
    }

    #[test]
    fn unknown_rules_survive_a_round_trip() {
        let mut data = BTreeMap::new();
        data.insert("mymod:x".to_owned(), Tag::String("on".to_owned()));
        data.insert("minecraft:max_minecart_speed".to_owned(), Tag::Int(8));
        data.insert("keep_inventory".to_owned(), Tag::Byte(1));
        data.insert("minecraft:random_tick_speed".to_owned(), Tag::Int(7));
        let rules = GameRules::from_tag(&Tag::Compound(data));
        assert!(rules.keep_inventory());
        assert_eq!(rules.int("random_tick_speed"), 7);
        let tag = rules.to_tag();
        assert_eq!(tag.get("mymod:x"), Some(&Tag::String("on".to_owned())));
        assert_eq!(tag.get("minecraft:max_minecart_speed"), Some(&Tag::Int(8)));
        assert_eq!(tag.get("keep_inventory"), None);
        assert_eq!(tag.get("minecraft:keep_inventory"), Some(&Tag::Byte(1)));
        assert_eq!(tag.as_compound().unwrap().len(), RULES.len() + 2);
        assert_eq!(GameRules::from_tag(&tag), rules);
    }

    #[test]
    fn out_of_range_values_are_clamped_and_wrong_types_use_the_default() {
        let mut data = BTreeMap::new();
        data.insert(
            "minecraft:max_snow_accumulation_height".to_owned(),
            Tag::Int(40),
        );
        data.insert("minecraft:random_tick_speed".to_owned(), Tag::Int(-5));
        data.insert(
            "minecraft:mob_griefing".to_owned(),
            Tag::String("false".to_owned()),
        );
        let rules = GameRules::from_tag(&Tag::Compound(data));
        assert_eq!(rules.int("max_snow_accumulation_height"), 8);
        assert_eq!(rules.int("random_tick_speed"), 0);
        assert!(rules.bool("mob_griefing"));
        assert_eq!(rules.to_tag().as_compound().unwrap().len(), RULES.len());
    }
}
