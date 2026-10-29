use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

use crate::lodestone::{COMPASS, TRACKER};
use crate::pack::{PackStack, ResourceId};
use crate::terrain::Dimension;

pub const DIAL_ITEMS: [&str; 3] = [
    "minecraft:clock",
    "minecraft:compass",
    "minecraft:recovery_compass",
];

const KEY_MARK: char = '#';
const LODESTONE_MARK: &str = "@lodestone:";
const NO_TARGET: &str = "none";
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
        Some((id, model)) => (item_of(id), Some(model)),
        None => (item_of(key), None),
    }
}

pub fn item_of(key: &str) -> &str {
    key.split_once(LODESTONE_MARK).map_or(key, |(id, _)| id)
}

type Tracked = Option<(Dimension, (i32, i32, i32))>;

pub fn stack_key(stack: &ItemStack) -> Cow<'_, str> {
    let Some(tracker) = crate::lodestone::tracker(stack) else {
        return Cow::Borrowed(&stack.id);
    };
    Cow::Owned(match tracker.target {
        Some(target) => {
            let (x, y, z) = target.pos;
            format!(
                "{}{LODESTONE_MARK}{}:{x},{y},{z}",
                stack.id, target.dimension
            )
        }
        None => format!("{}{LODESTONE_MARK}{NO_TARGET}", stack.id),
    })
}

fn lodestone_of(key: &str) -> Option<Tracked> {
    let rest = key.strip_prefix(COMPASS)?.strip_prefix(LODESTONE_MARK)?;
    let target = rest.rsplit_once(':').and_then(|(dimension, at)| {
        let mut coords = at.split(',').map(|v| v.parse::<i32>().ok());
        let pos = (coords.next()??, coords.next()??, coords.next()??);
        coords
            .next()
            .is_none()
            .then_some((Dimension::parse(dimension)?, pos))
    });
    Some(target)
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
    watched: Vec<String>,
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
            watched: Vec::new(),
        }
    }
}

impl Dials {
    pub fn key<'a>(&'a self, id: &'a str) -> &'a str {
        let item = item_of(id);
        self.keys
            .get(id)
            .or_else(|| self.keys.get(item))
            .map_or(item, String::as_str)
    }

    pub fn watch<'a>(&mut self, keys: impl IntoIterator<Item = &'a str>) {
        let mut watched: Vec<String> = keys
            .into_iter()
            .map(|key| crate::item_tint::split(key).0)
            .filter(|key| lodestone_of(key).is_some())
            .map(str::to_owned)
            .collect();
        watched.sort_unstable();
        watched.dedup();
        self.keys
            .retain(|key, _| lodestone_of(key).is_none() || watched.binary_search(key).is_ok());
        self.watched = watched;
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
        let watched = self.watched.clone();
        for _ in 0..ticks.clamp(1, MAX_TICKS_PER_STEP) {
            self.tick += 1;
            for id in DIAL_ITEMS {
                let Some(definition) = self.definition(packs, id) else {
                    continue;
                };
                if let Some(model) = self.leaf(&definition["model"], context, None) {
                    self.keys.insert(id.to_owned(), keyed(id, &model));
                }
            }
            for key in &watched {
                let Some(definition) = self.definition(packs, COMPASS) else {
                    break;
                };
                if let Some(model) = self.leaf(&definition["model"], context, lodestone_of(key)) {
                    self.keys.insert(key.clone(), keyed(COMPASS, &model));
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

    fn compass(&mut self, target: &str, context: &DialContext, lodestone: Option<Tracked>) -> f32 {
        let aim = match target {
            "spawn" => context.spawn,
            "recovery" => context.last_death,
            "lodestone" => lodestone.flatten(),
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

    fn value(
        &mut self,
        model: &Value,
        context: &DialContext,
        lodestone: Option<Tracked>,
    ) -> Option<f32> {
        let field = |key: &str| model.get(key).and_then(Value::as_str);
        match field("property")? {
            "minecraft:compass" => Some(self.compass(
                field("target").unwrap_or("none"),
                context,
                lodestone,
            )),
            "minecraft:time" => Some(self.time(field("source").unwrap_or("daytime"), context)),
            _ => None,
        }
    }

    fn leaf(
        &mut self,
        model: &Value,
        context: &DialContext,
        lodestone: Option<Tracked>,
    ) -> Option<String> {
        match model.get("type")?.as_str()? {
            "minecraft:model" => model.get("model")?.as_str().map(str::to_owned),
            "minecraft:condition" => {
                let field = |key: &str| model.get(key).and_then(Value::as_str);
                let tracker = field("property") == Some("minecraft:has_component")
                    && field("component") == Some(TRACKER)
                    && lodestone.is_some();
                let branch = if tracker { "on_true" } else { "on_false" };
                self.leaf(model.get(branch)?, context, lodestone)
            }
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
                self.leaf(chosen.or_else(|| model.get("fallback"))?, context, lodestone)
            }
            "minecraft:range_dispatch" => {
                let scale = model.get("scale").and_then(Value::as_f64).unwrap_or(1.0) as f32;
                let value = self.value(model, context, lodestone)? * scale;
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
                self.leaf(picked, context, lodestone)
            }
            "minecraft:composite" => model
                .get("models")?
                .as_array()?
                .iter()
                .find_map(|part| self.leaf(part, context, lodestone)),
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
            "on_true": {"type": "minecraft:range_dispatch", "property": "minecraft:compass", "scale": 32.0, "target": "lodestone", "entries": entries}
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
        settle_tracked(dials, model, (context, None), ticks)
    }

    fn settle_tracked(
        dials: &mut Dials,
        model: &Value,
        (context, lodestone): (&DialContext, Option<Tracked>),
        ticks: usize,
    ) -> Vec<String> {
        (0..ticks)
            .map(|_| {
                dials.tick += 1;
                dials.leaf(model, context, lodestone).unwrap()
            })
            .collect()
    }

    fn distinct(mut shown: Vec<String>) -> usize {
        shown.sort();
        shown.dedup();
        shown.len()
    }

    fn lodestone_compass(target: Option<crate::lodestone::Target>) -> ItemStack {
        let compass = ItemStack::new(COMPASS, 1);
        match target {
            Some(target) => crate::lodestone::with_tracker(&compass, target),
            None => {
                let mut lost = compass;
                crate::lodestone::set_tracker(
                    &mut lost,
                    &crate::lodestone::Tracker {
                        target: None,
                        tracked: true,
                    },
                );
                lost
            }
        }
    }

    #[test]
    fn a_lodestone_compass_points_at_its_lodestone_and_spins_without_one_or_elsewhere() {
        let mut dials = Dials::default();
        let model = compass();
        let north = Some(Some((Dimension::Overworld, (0, 64, -100))));
        let facing_south = context(Dimension::Overworld, 0.0);
        let shown = settle_tracked(&mut dials, &model, (&facing_south, north), 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/compass_00"),
            "the lodestone behind, though the spawn is ahead: {:?}",
            &shown[250..]
        );
        let shown = settle(&mut dials, &model, &facing_south, 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/compass_16"),
            "a plain compass ignores the branch and points at the spawn"
        );
        let facing_north = context(Dimension::Overworld, 180.0);
        let shown = settle_tracked(&mut dials, &model, (&facing_north, north), 300);
        assert!(
            shown[250..]
                .iter()
                .all(|frame| frame == "minecraft:item/compass_16"),
            "facing it: straight ahead {:?}",
            &shown[250..]
        );
        let nether = Some(Some((Dimension::Nether, (0, 64, -100))));
        let shown = settle_tracked(&mut dials, &model, (&facing_north, nether), 200);
        assert!(distinct(shown) > 8, "another dimension's lodestone: it spins");
        let shown = settle_tracked(&mut dials, &model, (&facing_north, Some(None)), 200);
        assert!(distinct(shown) > 8, "no lodestone left: it spins");
        let on_top = Some(Some((Dimension::Overworld, (0, 63, 0))));
        let standing = DialContext {
            feet: [0.5, 63.5, 0.5],
            ..facing_north
        };
        let shown = settle_tracked(&mut dials, &model, (&standing, on_top), 200);
        assert!(distinct(shown) > 8, "standing in its centre: it spins");
    }

    #[test]
    fn a_lodestone_compass_has_its_own_key_turned_each_tick_while_it_is_shown() {
        use crate::lodestone::Target;
        let packs = PackStack::open(Vec::new()).unwrap();
        let mut dials = Dials::default();
        dials.definitions.insert(
            COMPASS,
            Some(Arc::new(json!({ "model": compass() }))),
        );
        let plain = ItemStack::new(COMPASS, 1);
        assert!(matches!(stack_key(&plain), Cow::Borrowed("minecraft:compass")));
        let north = lodestone_compass(Some(Target::new(Dimension::Overworld, (0, 64, -100))));
        let key = stack_key(&north).into_owned();
        assert_eq!(key, "minecraft:compass@lodestone:minecraft:overworld:0,64,-100");
        let lost = stack_key(&lodestone_compass(None)).into_owned();
        assert_eq!(lost, "minecraft:compass@lodestone:none");
        assert_eq!(lodestone_of(&key), Some(Some((Dimension::Overworld, (0, 64, -100)))));
        assert_eq!(lodestone_of(&lost), Some(None));
        assert_eq!(lodestone_of("minecraft:compass@lodestone:minecraft:the_moon:1,2,3"), Some(None));
        assert_eq!(lodestone_of("minecraft:compass"), None);
        assert_eq!(item_of(&key), COMPASS);
        assert_eq!(dials.key(&key), COMPASS, "nothing stepped yet: the plain item");
        dials.watch([key.as_str(), "minecraft:stone", "minecraft:potion~f82423"]);
        let facing_south = context(Dimension::Overworld, 0.0);
        for _ in 0..300 {
            dials.step(0.05, &packs, &facing_south);
        }
        assert_eq!(dials.key(&key), "minecraft:compass#minecraft:item/compass_00");
        assert_eq!(
            dials.key(COMPASS),
            "minecraft:compass#minecraft:item/compass_16",
            "the plain compass keeps its own needle"
        );
        assert_eq!(dials.watched, [key.clone()]);
        assert_eq!(
            dials.key(&lost),
            "minecraft:compass#minecraft:item/compass_16",
            "a key not stepped yet shows the plain compass's frame"
        );
        dials.watch([lost.as_str()]);
        assert_eq!(dials.key(&key), "minecraft:compass#minecraft:item/compass_16", "no longer shown: dropped");
        assert!(!dials.keys.contains_key(&key));
        dials.step(0.1, &packs, &facing_south);
        assert!(split(dials.key(&lost)).1.is_some_and(|model| model.starts_with("minecraft:item/compass_")));
        assert!(dials.keys.contains_key(&lost));
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
        let north = crate::lodestone::Target::new(Dimension::Overworld, (0, 64, -100));
        let lodestone = stack_key(&lodestone_compass(Some(north))).into_owned();
        dials.watch([lodestone.as_str()]);
        for _ in 0..300 {
            dials.step(0.05, &packs, &overworld);
        }
        assert_eq!(
            dials.key("minecraft:compass"),
            "minecraft:compass#minecraft:item/compass_00"
        );
        assert_eq!(
            dials.key(&lodestone),
            "minecraft:compass#minecraft:item/compass_16",
            "the lodestone ahead, the spawn behind"
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
