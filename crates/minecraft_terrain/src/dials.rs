use std::collections::HashMap;
use std::sync::Arc;

use serde_json::Value;

use crate::pack::{PackStack, ResourceId};
use crate::terrain::Dimension;

pub const DIAL_ITEMS: [&str; 3] = [
    "minecraft:clock",
    "minecraft:compass",
    "minecraft:recovery_compass",
];

const KEY_MARK: char = '#';
const TICK_SECONDS: f64 = 0.05;
const MAX_TICKS_PER_STEP: u32 = 20;
const COMPASS_WOBBLE: f32 = 0.8;
const CLOCK_WOBBLE: f32 = 0.9;
const TARGET_MIN_DISTANCE_SQR: f64 = 1.0e-5;

pub fn keyed(id: &str, model: &str) -> String {
    format!("{id}{KEY_MARK}{model}")
}

pub fn split(key: &str) -> (&str, Option<&str>) {
    match key.split_once(KEY_MARK) {
        Some((id, model)) => (id, Some(model)),
        None => (key, None),
    }
}

pub fn positive_modulo(value: f32, modulus: f32) -> f32 {
    (value % modulus + modulus) % modulus
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Needle {
    rotation: f32,
    delta: f32,
    last: Option<u64>,
}

impl Needle {
    pub fn update(&mut self, tick: u64, target: f32, factor: f32) {
        if self.last == Some(tick) {
            return;
        }
        self.last = Some(tick);
        let offset = positive_modulo(target - self.rotation + 0.5, 1.0) - 0.5;
        self.delta += offset * 0.1;
        self.delta *= factor;
        self.rotation = positive_modulo(self.rotation + self.delta, 1.0);
    }

    pub fn rotation(&self) -> f32 {
        self.rotation
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DialContext {
    pub dimension: Dimension,
    pub feet: [f64; 3],
    pub yaw: f32,
    pub spawn: Option<(Dimension, (i32, i32, i32))>,
    pub last_death: Option<(Dimension, (i32, i32, i32))>,
    pub sun_angle: f32,
}

pub struct Dials {
    clock: f64,
    tick: u64,
    random: u64,
    needles: HashMap<String, (Needle, Needle)>,
    definitions: HashMap<&'static str, Option<Arc<Value>>>,
    keys: HashMap<String, String>,
}

impl Default for Dials {
    fn default() -> Self {
        Self {
            clock: 0.0,
            tick: 0,
            random: 0x2545_f491_4f6c_dd1d,
            needles: HashMap::new(),
            definitions: HashMap::new(),
            keys: HashMap::new(),
        }
    }
}

impl Dials {
    pub fn key<'a>(&'a self, id: &'a str) -> &'a str {
        self.keys.get(id).map_or(id, String::as_str)
    }

    pub fn keys(&self) -> &HashMap<String, String> {
        &self.keys
    }

    pub fn step(&mut self, seconds: f64, packs: &PackStack, context: &DialContext) {
        self.clock += seconds.max(0.0);
        let mut ticks = 0;
        while self.clock >= TICK_SECONDS {
            self.clock -= TICK_SECONDS;
            ticks += 1;
        }
        if ticks == 0 && !self.keys.is_empty() {
            return;
        }
        for _ in 0..ticks.clamp(1, MAX_TICKS_PER_STEP) {
            self.tick += 1;
            for id in DIAL_ITEMS {
                let Some(definition) = self.definition(packs, id) else {
                    continue;
                };
                if let Some(model) = self.leaf(&definition["model"], context) {
                    self.keys.insert(id.to_owned(), keyed(id, &model));
                }
            }
        }
        self.clock = self.clock.min(TICK_SECONDS);
    }

    fn definition(&mut self, packs: &PackStack, id: &'static str) -> Option<Arc<Value>> {
        self.definitions
            .entry(id)
            .or_insert_with(|| {
                let resource = ResourceId::parse(id).ok()?;
                packs
                    .item_definition(&resource)
                    .ok()
                    .flatten()
                    .map(Arc::new)
            })
            .clone()
    }

    fn random_float(&mut self) -> f32 {
        self.random ^= self.random << 13;
        self.random ^= self.random >> 7;
        self.random ^= self.random << 17;
        (self.random >> 40) as f32 / (1u64 << 24) as f32
    }

    fn needle(&mut self, name: String) -> &mut (Needle, Needle) {
        self.needles.entry(name).or_default()
    }

    fn compass(&mut self, target: &str, context: &DialContext) -> f32 {
        let aim = match target {
            "spawn" => context.spawn,
            "recovery" => context.last_death,
            _ => None,
        }
        .filter(|(dimension, (x, y, z))| {
            let centre = [
                f64::from(*x) + 0.5,
                f64::from(*y) + 0.5,
                f64::from(*z) + 0.5,
            ];
            let distance: f64 = (0..3).map(|i| (centre[i] - context.feet[i]).powi(2)).sum();
            *dimension == context.dimension && distance >= TARGET_MIN_DISTANCE_SQR
        });
        let tick = self.tick;
        match aim {
            Some((_, (x, _, z))) => {
                let angle = ((f64::from(z) + 0.5 - context.feet[2])
                    .atan2(f64::from(x) + 0.5 - context.feet[0])
                    / std::f64::consts::TAU) as f32;
                let yaw = positive_modulo(context.yaw / 360.0, 1.0);
                let (needle, _) = self.needle(format!("compass:{target}"));
                needle.update(tick, 0.5 - (yaw - 0.25), COMPASS_WOBBLE);
                positive_modulo(angle + needle.rotation(), 1.0)
            }
            None => {
                let spin = self.random_float();
                let (_, needle) = self.needle(format!("compass:{target}"));
                needle.update(tick, spin, COMPASS_WOBBLE);
                positive_modulo(needle.rotation(), 1.0)
            }
        }
    }

    fn time(&mut self, source: &str, context: &DialContext) -> f32 {
        let target = match source {
            "daytime" => context.sun_angle / 360.0,
            "random" => self.random_float(),
            _ => 0.0,
        };
        let tick = self.tick;
        let (needle, _) = self.needle(format!("time:{source}"));
        needle.update(tick, target, CLOCK_WOBBLE);
        needle.rotation()
    }

    fn value(&mut self, model: &Value, context: &DialContext) -> Option<f32> {
        let field = |key: &str| model.get(key).and_then(Value::as_str);
        match field("property")? {
            "minecraft:compass" => Some(self.compass(field("target").unwrap_or("none"), context)),
            "minecraft:time" => Some(self.time(field("source").unwrap_or("daytime"), context)),
            _ => None,
        }
    }

    fn leaf(&mut self, model: &Value, context: &DialContext) -> Option<String> {
        match model.get("type")?.as_str()? {
            "minecraft:model" => model.get("model")?.as_str().map(str::to_owned),
            "minecraft:condition" => self.leaf(model.get("on_false")?, context),
            "minecraft:select" => {
                let dimension = context.dimension.dimension_type();
                let chosen = (model.get("property").and_then(Value::as_str)
                    == Some("minecraft:context_dimension"))
                .then(|| {
                    model
                        .get("cases")?
                        .as_array()?
                        .iter()
                        .find(|case| match &case["when"] {
                            Value::String(when) => when == dimension,
                            Value::Array(whens) => whens.iter().any(|when| when == dimension),
                            _ => false,
                        })
                })
                .flatten()
                .map(|case| &case["model"]);
                self.leaf(chosen.or_else(|| model.get("fallback"))?, context)
            }
            "minecraft:range_dispatch" => {
                let scale = model.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                let value = self.value(model, context)? * scale;
                let entries = model.get("entries")?.as_array()?;
                let picked = entries
                    .iter()
                    .rev()
                    .find(|entry| {
                        entry["threshold"]
                            .as_f64()
                            .is_some_and(|threshold| threshold as f32 <= value)
                    })
                    .map(|entry| &entry["model"])
                    .or_else(|| model.get("fallback"))?;
                self.leaf(picked, context)
            }
            "minecraft:composite" => model
                .get("models")?
                .as_array()?
                .iter()
                .find_map(|part| self.leaf(part, context)),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn context(dimension: Dimension, yaw: f32) -> DialContext {
        DialContext {
            dimension,
            feet: [0.5, 64.0, 0.5],
            yaw,
            spawn: Some((Dimension::Overworld, (0, 64, 100))),
            last_death: None,
            sun_angle: 90.0,
        }
    }

    fn compass() -> Value {
        let entries: Vec<Value> = (0..=32)
            .map(|i| {
                json!({
                    "threshold": if i == 0 { 0.0 } else { f64::from(i) - 0.5 },
                    "model": {"type": "minecraft:model", "model": format!("minecraft:item/compass_{:02}", (i + 16) % 32)}
                })
            })
            .collect();
        json!({
            "type": "minecraft:condition",
            "property": "minecraft:has_component",
            "component": "minecraft:lodestone_tracker",
            "on_false": {"type": "minecraft:range_dispatch", "property": "minecraft:compass", "scale": 32.0, "target": "spawn", "entries": entries},
            "on_true": {"type": "minecraft:model", "model": "minecraft:item/compass_16"}
        })
    }

    fn clock() -> Value {
        let dial = |source: &str| {
            let entries: Vec<Value> = (0..=64)
                .map(|i| {
                    json!({
                        "threshold": if i == 0 { 0.0 } else { f64::from(i) - 0.5 },
                        "model": {"type": "minecraft:model", "model": format!("minecraft:item/clock_{:02}", i % 64)}
                    })
                })
                .collect();
            json!({"type": "minecraft:range_dispatch", "property": "minecraft:time", "scale": 64.0, "source": source, "entries": entries})
        };
        json!({
            "type": "minecraft:select",
            "property": "minecraft:context_dimension",
            "cases": [{"when": "minecraft:overworld", "model": dial("daytime")}],
            "fallback": dial("random")
        })
    }

    fn settle(
        dials: &mut Dials,
        model: &Value,
        context: &DialContext,
        ticks: usize,
    ) -> Vec<String> {
        (0..ticks)
            .map(|_| {
                dials.tick += 1;
                dials.leaf(model, context).unwrap()
            })
            .collect()
    }

    #[test]
    fn a_needle_swings_toward_its_target_and_settles_there() {
        let mut needle = Needle::default();
        needle.update(1, 0.25, COMPASS_WOBBLE);
        assert!((needle.rotation() - 0.02).abs() < 1.0e-6);
        needle.update(1, 0.25, COMPASS_WOBBLE);
        assert!((needle.rotation() - 0.02).abs() < 1.0e-6, "once a tick");
        for tick in 2..400 {
            needle.update(tick, 0.25, COMPASS_WOBBLE);
        }
        assert!((needle.rotation() - 0.25).abs() < 1.0e-4);
        assert!((positive_modulo(-0.25, 1.0) - 0.75).abs() < 1.0e-6);
    }

    #[test]
    fn a_compass_points_at_the_overworld_spawn_and_spins_in_the_nether() {
        let mut dials = Dials::default();
        let model = compass();
        let facing_south = context(Dimension::Overworld, 0.0);
        let shown = settle(&mut dials, &model, &facing_south, 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/compass_16"),
            "facing the spawn straight ahead: {:?}",
            &shown[250..]
        );
        let facing_north = context(Dimension::Overworld, 180.0);
        let shown = settle(&mut dials, &model, &facing_north, 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/compass_00"),
            "the spawn behind: {:?}",
            &shown[250..]
        );
        let nether = context(Dimension::Nether, 0.0);
        let shown = settle(&mut dials, &model, &nether, 200);
        let mut distinct = shown.clone();
        distinct.sort();
        distinct.dedup();
        assert!(distinct.len() > 8, "it wanders: {distinct:?}");
        let at_the_spawn = DialContext {
            feet: [0.5, 64.5, 100.5],
            ..facing_south
        };
        let shown = settle(&mut dials, &model, &at_the_spawn, 200);
        let mut distinct = shown.clone();
        distinct.sort();
        distinct.dedup();
        assert!(
            distinct.len() > 8,
            "standing on the spawn it spins: {distinct:?}"
        );
    }

    #[test]
    fn a_clock_tells_the_sun_in_the_overworld_and_spins_elsewhere() {
        let mut dials = Dials::default();
        let model = clock();
        let overworld = context(Dimension::Overworld, 0.0);
        let shown = settle(&mut dials, &model, &overworld, 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/clock_16")
        );
        let noon = DialContext {
            sun_angle: 0.0,
            ..overworld
        };
        let shown = settle(&mut dials, &model, &noon, 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/clock_00")
        );
        let end = context(Dimension::End, 0.0);
        let shown = settle(&mut dials, &model, &end, 200);
        let mut distinct = shown.clone();
        distinct.sort();
        distinct.dedup();
        assert!(distinct.len() > 8, "it wanders: {distinct:?}");
    }

    #[test]
    fn vanilla_compasses_and_clocks_get_their_frame_as_icon_and_held_model() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let pack = std::path::Path::new(&root).join("resourcepacks/local/minecraft-26.3");
        if !pack.is_dir() {
            return;
        }
        let packs = PackStack::open(vec![pack]).unwrap();
        let mut dials = Dials::default();
        let overworld = context(Dimension::Overworld, 180.0);
        for _ in 0..300 {
            dials.step(0.05, &packs, &overworld);
        }
        assert_eq!(
            dials.key("minecraft:compass"),
            "minecraft:compass#minecraft:item/compass_00"
        );
        assert_eq!(
            dials.key("minecraft:clock"),
            "minecraft:clock#minecraft:item/clock_16"
        );
        assert!(
            dials
                .key("minecraft:recovery_compass")
                .starts_with("minecraft:recovery_compass#minecraft:item/recovery_compass_")
        );
        assert_eq!(dials.key("minecraft:stone"), "minecraft:stone");
        let clock = crate::item_icon::SpriteClock::at(0);
        let behind =
            crate::item_icons::item_icon_at(&packs, dials.key("minecraft:compass"), 16, &clock)
                .unwrap()
                .unwrap();
        assert!(clock.animated(), "a dial's icon is redrawn as it turns");
        let ahead = crate::item_icons::item_icon_at(
            &packs,
            &keyed("minecraft:compass", "minecraft:item/compass_16"),
            16,
            &clock,
        )
        .unwrap()
        .unwrap();
        assert_ne!(behind, ahead);
        let model = crate::model::resolve_item_leaf(
            &packs,
            &ResourceId::parse("minecraft:compass").unwrap(),
            "minecraft:item/compass_00",
        )
        .unwrap();
        assert!(!model.elements.is_empty());
    }

    #[test]
    fn a_dial_key_carries_the_model_it_shows() {
        let key = keyed("minecraft:compass", "minecraft:item/compass_07");
        assert_eq!(key, "minecraft:compass#minecraft:item/compass_07");
        assert_eq!(
            split(&key),
            ("minecraft:compass", Some("minecraft:item/compass_07"))
        );
        assert_eq!(split("minecraft:stone"), ("minecraft:stone", None));
        let dials = Dials::default();
        assert_eq!(dials.key("minecraft:compass"), "minecraft:compass");
    }
}
