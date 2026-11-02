use glam::Mat4;
use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

use crate::pack::ResourceId;
use crate::special_models::{self, DYES};

pub const KEY_MARK: char = '|';
pub const MAX_LAYERS: usize = 16;
pub const CROSSBOW_CHARGE_TICKS: u32 = 25;

const CHARGED_PROJECTILES: &str = "minecraft:charged_projectiles";
const BASE_COLOR: &str = "minecraft:base_color";
const BANNER_PATTERNS: &str = "minecraft:banner_patterns";
const FIREWORK_ROCKET: &str = "minecraft:firework_rocket";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Charge {
    #[default]
    None,
    Arrow,
    Rocket,
}

impl Charge {
    pub fn of(components: Option<&Value>) -> Self {
        let projectiles = components
            .and_then(|components| components.get(CHARGED_PROJECTILES))
            .and_then(Value::as_array)
            .map_or(&[][..], Vec::as_slice);
        if projectiles.is_empty() {
            Self::None
        } else if projectiles
            .iter()
            .any(|projectile| item_id(projectile).as_deref() == Some(FIREWORK_ROCKET))
        {
            Self::Rocket
        } else {
            Self::Arrow
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Arrow => "arrow",
            Self::Rocket => "rocket",
        }
    }

    fn named(name: &str) -> Self {
        [Self::Arrow, Self::Rocket]
            .into_iter()
            .find(|charge| charge.name() == name)
            .unwrap_or_default()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ItemState {
    pub using: bool,
    pub charge: Charge,
    pub use_ticks: u32,
    pub charge_duration: u32,
}

impl ItemState {
    pub fn plain(using: bool) -> Self {
        Self {
            using,
            charge: Charge::None,
            use_ticks: 0,
            charge_duration: CROSSBOW_CHARGE_TICKS,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ItemLook {
    pub charge: Charge,
    pub use_ticks: Option<u32>,
    pub base: Option<&'static str>,
    pub layers: Vec<(String, &'static str)>,
}

impl ItemLook {
    pub fn of(components: Option<&Value>, use_ticks: Option<u32>) -> Self {
        let component = |name: &str| components.and_then(|components| components.get(name));
        Self {
            charge: Charge::of(components),
            use_ticks,
            base: component(BASE_COLOR).and_then(Value::as_str).and_then(dye),
            layers: component(BANNER_PATTERNS)
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .take(MAX_LAYERS)
                .filter_map(layer)
                .collect(),
        }
    }

    pub fn parse(suffix: &str) -> Self {
        let mut look = Self::default();
        for part in suffix.split(KEY_MARK) {
            let Some((name, value)) = part.split_once('=') else {
                continue;
            };
            match name {
                "charge" => look.charge = Charge::named(value),
                "use" => look.use_ticks = value.parse().ok(),
                "base" => look.base = dye(value),
                "layer" => look.layers.extend(
                    value
                        .split_once(',')
                        .and_then(|(color, asset)| Some((asset.to_owned(), dye(color)?))),
                ),
                _ => {}
            }
        }
        look
    }

    pub fn suffix(&self) -> String {
        let charge =
            (self.charge != Charge::None).then(|| format!("charge={}", self.charge.name()));
        let ticks = self.use_ticks.map(|ticks| format!("use={ticks}"));
        let base = self.base.map(|base| format!("base={base}"));
        let layers = self
            .layers
            .iter()
            .map(|(asset, color)| format!("layer={color},{asset}"));
        charge
            .into_iter()
            .chain(ticks)
            .chain(base)
            .chain(layers)
            .map(|part| format!("{KEY_MARK}{part}"))
            .collect()
    }

    pub fn state(&self) -> ItemState {
        ItemState {
            using: self.use_ticks.is_some(),
            charge: self.charge,
            use_ticks: self.use_ticks.unwrap_or(0),
            charge_duration: CROSSBOW_CHARGE_TICKS,
        }
    }

    pub fn leaf<'a>(&self, model: &'a Value, context: Option<&str>) -> Option<&'a str> {
        let state = self.state();
        if state == ItemState::plain(false) {
            return None;
        }
        match special_models::item_leaves_with(model, &state, context).as_slice() {
            [leaf] if leaf.special.is_none() && leaf.transform == Mat4::IDENTITY => {
                Some(leaf.model)
            }
            _ => None,
        }
    }
}

pub fn keyed(base: &str, look: &ItemLook) -> String {
    format!("{base}{}", look.suffix())
}

pub fn held_key(stack: &ItemStack, use_ticks: Option<i32>) -> String {
    let use_ticks = use_ticks.and_then(|ticks| u32::try_from(ticks).ok());
    keyed(
        &stack.id,
        &ItemLook::of(stack.components.as_ref(), use_ticks),
    )
}

pub fn split(key: &str) -> (&str, &str) {
    key.find(KEY_MARK).map_or((key, ""), |at| key.split_at(at))
}

fn dye(name: &str) -> Option<&'static str> {
    DYES.iter()
        .find(|(dye, _)| *dye == name)
        .map(|&(dye, _)| dye)
}

fn item_id(stack: &Value) -> Option<String> {
    let id = stack.as_str().or_else(|| stack.get("id")?.as_str())?;
    ResourceId::parse(id).ok().map(|id| id.key())
}

fn layer(layer: &Value) -> Option<(String, &'static str)> {
    let pattern = layer.get("pattern")?;
    let asset = pattern
        .as_str()
        .or_else(|| pattern.get("asset_id")?.as_str())?;
    let asset = ResourceId::parse(asset).ok()?.key();
    Some((asset, dye(layer.get("color")?.as_str()?)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dials, item_tint};
    use serde_json::json;

    fn stack(id: &str, components: Value) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.components = Some(components);
        stack
    }

    fn crossbow() -> Value {
        json!({
            "type": "minecraft:select",
            "property": "minecraft:charge_type",
            "cases": [
                {"when": "arrow", "model": {"type": "minecraft:model", "model": "minecraft:item/crossbow_arrow"}},
                {"when": "rocket", "model": {"type": "minecraft:model", "model": "minecraft:item/crossbow_firework"}}
            ],
            "fallback": {
                "type": "minecraft:condition",
                "property": "minecraft:using_item",
                "on_false": {"type": "minecraft:model", "model": "minecraft:item/crossbow"},
                "on_true": {
                    "type": "minecraft:range_dispatch",
                    "property": "minecraft:crossbow/pull",
                    "entries": [
                        {"threshold": 0.58, "model": {"type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_1"}},
                        {"threshold": 1.0, "model": {"type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_2"}}
                    ],
                    "fallback": {"type": "minecraft:model", "model": "minecraft:item/crossbow_pulling_0"}
                }
            }
        })
    }

    #[test]
    fn a_look_reads_the_charge_the_patterns_and_the_base_colour() {
        assert_eq!(ItemLook::of(None, None), ItemLook::default());
        assert_eq!(ItemLook::default().suffix(), "");
        for (projectiles, charge) in [
            (json!([]), Charge::None),
            (json!([{"id": "minecraft:arrow"}]), Charge::Arrow),
            (json!(["minecraft:spectral_arrow"]), Charge::Arrow),
            (
                json!([{"id": "minecraft:arrow"}, {"id": "minecraft:firework_rocket", "count": 1}]),
                Charge::Rocket,
            ),
            (json!([{"id": "firework_rocket"}]), Charge::Rocket),
        ] {
            let components = json!({ "minecraft:charged_projectiles": projectiles });
            assert_eq!(Charge::of(Some(&components)), charge, "{projectiles}");
        }
        let shield = json!({
            "minecraft:base_color": "red",
            "minecraft:banner_patterns": [
                {"pattern": "minecraft:cross", "color": "yellow"},
                {"pattern": {"asset_id": "globe", "translation_key": "block.minecraft.banner.globe"}, "color": "light_blue"},
                {"pattern": "minecraft:border", "color": "purpl"},
                {"pattern": "Not An Id", "color": "black"},
                {"color": "black"},
                {"pattern": "minecraft:stripe_top", "color": "black"}
            ]
        });
        let look = ItemLook::of(Some(&shield), None);
        assert_eq!(look.base, Some("red"));
        assert_eq!(
            look.layers,
            [
                ("minecraft:cross".to_owned(), "yellow"),
                ("minecraft:globe".to_owned(), "light_blue"),
                ("minecraft:stripe_top".to_owned(), "black")
            ]
        );
        assert_eq!(
            look.suffix(),
            "|base=red|layer=yellow,minecraft:cross|layer=light_blue,minecraft:globe|layer=black,minecraft:stripe_top"
        );
        assert_eq!(ItemLook::parse(&look.suffix()), look);
        let many: Vec<Value> = (0..20)
            .map(|i| json!({"pattern": format!("minecraft:p{i}"), "color": "white"}))
            .collect();
        let banner = ItemLook::of(Some(&json!({ "minecraft:banner_patterns": many })), None);
        assert_eq!(banner.layers.len(), MAX_LAYERS);
        assert_eq!(banner.layers[15].0, "minecraft:p15");
        let bad = json!({"minecraft:base_color": "purpl", "minecraft:banner_patterns": {"pattern": "minecraft:cross"}});
        assert_eq!(ItemLook::of(Some(&bad), None), ItemLook::default());
        let charging = ItemLook::of(None, Some(12));
        assert_eq!(charging.suffix(), "|use=12");
        assert_eq!(ItemLook::parse("|use=12"), charging);
        let loaded = ItemLook::of(
            Some(&json!({"minecraft:charged_projectiles": [{"id": "minecraft:arrow"}]})),
            Some(3),
        );
        assert_eq!(loaded.suffix(), "|charge=arrow|use=3");
        assert_eq!(ItemLook::parse(&loaded.suffix()), loaded);
        assert_eq!(
            ItemLook::parse("|charge=none|use=x|base=purpl|layer=red|layer=,a|junk|=1"),
            ItemLook::default()
        );
    }

    #[test]
    fn a_look_sits_after_the_dial_and_before_the_colour_and_splits_back() {
        let loaded = stack(
            "minecraft:crossbow",
            json!({"minecraft:charged_projectiles": [{"id": "minecraft:firework_rocket"}]}),
        );
        let key = item_tint::of_stack(&loaded);
        assert_eq!(key, "minecraft:crossbow|charge=rocket");
        assert_eq!(item_tint::split(&key), (key.as_str(), None));
        assert_eq!(split(&key), ("minecraft:crossbow", "|charge=rocket"));
        assert_eq!(ItemLook::parse(split(&key).1).charge, Charge::Rocket);
        assert_eq!(dials::split(split(&key).0), ("minecraft:crossbow", None));
        assert_eq!(
            item_tint::of_stack(&ItemStack::new("minecraft:crossbow", 1)),
            "minecraft:crossbow"
        );
        let potion = stack(
            "minecraft:potion",
            json!({
                "minecraft:potion_contents": {"potion": "minecraft:healing"},
                "minecraft:banner_patterns": [{"pattern": "minecraft:creeper", "color": "black"}]
            }),
        );
        let key = item_tint::of_stack(&potion);
        assert_eq!(key, "minecraft:potion|layer=black,minecraft:creeper~f82423");
        let (rest, color) = item_tint::split(&key);
        assert_eq!(color, Some(0xF82423));
        let (base, suffix) = split(rest);
        assert_eq!(dials::split(base), ("minecraft:potion", None));
        assert_eq!(
            ItemLook::parse(suffix),
            ItemLook::of(potion.components.as_ref(), None)
        );
        let armour = stack(
            "minecraft:leather_horse_armor",
            json!({"minecraft:dyed_color": 0xB02E26, "minecraft:base_color": "blue"}),
        );
        let key = item_tint::of_stack(&armour);
        assert_eq!(key, "minecraft:leather_horse_armor|base=blue~b02e26");
        let (rest, color) = item_tint::split(&key);
        assert_eq!(
            (split(rest), color),
            (
                ("minecraft:leather_horse_armor", "|base=blue"),
                Some(0xB02E26)
            )
        );
        let compass = stack(
            "minecraft:compass",
            json!({"minecraft:charged_projectiles": ["minecraft:arrow"]}),
        );
        let dial = dials::keyed("minecraft:compass", "minecraft:item/compass_16");
        let held = item_tint::of_stack_at(&dial, &compass);
        assert_eq!(held, format!("{dial}|charge=arrow"));
        let (rest, color) = item_tint::split(&held);
        assert_eq!(color, None);
        let (base, suffix) = split(rest);
        assert_eq!((base, suffix), (dial.as_str(), "|charge=arrow"));
        assert_eq!(
            dials::split(base),
            ("minecraft:compass", Some("minecraft:item/compass_16"))
        );
        assert_eq!(dials::split(rest).0, "minecraft:compass");
        let keyed_again = item_tint::key(&keyed(base, &ItemLook::parse(suffix)), color);
        assert_eq!(keyed_again, held);
    }

    #[test]
    fn a_look_picks_a_plain_leaf_only_when_it_holds_a_state() {
        let crossbow = crossbow();
        assert_eq!(ItemLook::default().leaf(&crossbow, None), None);
        let patterned = ItemLook::of(Some(&json!({"minecraft:base_color": "red"})), None);
        assert_eq!(patterned.leaf(&crossbow, None), None);
        let loaded = ItemLook::of(
            Some(&json!({"minecraft:charged_projectiles": [{"id": "minecraft:arrow"}]})),
            None,
        );
        assert_eq!(
            loaded.leaf(&crossbow, Some("gui")),
            Some("minecraft:item/crossbow_arrow")
        );
        for (ticks, model) in [
            (0, "minecraft:item/crossbow_pulling_0"),
            (15, "minecraft:item/crossbow_pulling_1"),
            (25, "minecraft:item/crossbow_pulling_2"),
        ] {
            assert_eq!(
                ItemLook::of(None, Some(ticks)).leaf(&crossbow, None),
                Some(model)
            );
        }
        let shield = json!({
            "type": "minecraft:condition",
            "property": "minecraft:using_item",
            "on_false": {"type": "minecraft:special", "base": "minecraft:item/shield", "model": {"type": "minecraft:shield"}},
            "on_true": {"type": "minecraft:special", "base": "minecraft:item/shield_blocking", "model": {"type": "minecraft:shield"}}
        });
        assert_eq!(ItemLook::of(None, Some(3)).leaf(&shield, None), None);
    }
}
