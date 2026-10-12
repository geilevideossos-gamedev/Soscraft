use minecraftoss_entities::effects::{EffectInstance, MobEffect};
use minecraftoss_player::survival::{EffectKind, SurvivalStatus};
use serde_json::Value;

pub const WATER: &str = "minecraft:water";
pub const DEFAULT_COLOR: u32 = 0x385DC6;
pub const CONTENTS: &str = "minecraft:potion_contents";
pub const DURATION_SCALE: &str = "minecraft:potion_duration_scale";

type Entry = (&'static str, i32, i32);

const POTIONS: [(&str, &[Entry]); 46] = [
    ("water", &[]),
    ("mundane", &[]),
    ("thick", &[]),
    ("awkward", &[]),
    ("night_vision", &[("night_vision", 3600, 0)]),
    ("long_night_vision", &[("night_vision", 9600, 0)]),
    ("invisibility", &[("invisibility", 3600, 0)]),
    ("long_invisibility", &[("invisibility", 9600, 0)]),
    ("leaping", &[("jump_boost", 3600, 0)]),
    ("long_leaping", &[("jump_boost", 9600, 0)]),
    ("strong_leaping", &[("jump_boost", 1800, 1)]),
    ("fire_resistance", &[("fire_resistance", 3600, 0)]),
    ("long_fire_resistance", &[("fire_resistance", 9600, 0)]),
    ("swiftness", &[("speed", 3600, 0)]),
    ("long_swiftness", &[("speed", 9600, 0)]),
    ("strong_swiftness", &[("speed", 1800, 1)]),
    ("slowness", &[("slowness", 1800, 0)]),
    ("long_slowness", &[("slowness", 4800, 0)]),
    ("strong_slowness", &[("slowness", 400, 3)]),
    (
        "turtle_master",
        &[("slowness", 400, 3), ("resistance", 400, 2)],
    ),
    (
        "long_turtle_master",
        &[("slowness", 800, 3), ("resistance", 800, 2)],
    ),
    (
        "strong_turtle_master",
        &[("slowness", 400, 5), ("resistance", 400, 3)],
    ),
    ("water_breathing", &[("water_breathing", 3600, 0)]),
    ("long_water_breathing", &[("water_breathing", 9600, 0)]),
    ("healing", &[("instant_health", 1, 0)]),
    ("strong_healing", &[("instant_health", 1, 1)]),
    ("harming", &[("instant_damage", 1, 0)]),
    ("strong_harming", &[("instant_damage", 1, 1)]),
    ("poison", &[("poison", 900, 0)]),
    ("long_poison", &[("poison", 1800, 0)]),
    ("strong_poison", &[("poison", 432, 1)]),
    ("regeneration", &[("regeneration", 900, 0)]),
    ("long_regeneration", &[("regeneration", 1800, 0)]),
    ("strong_regeneration", &[("regeneration", 450, 1)]),
    ("strength", &[("strength", 3600, 0)]),
    ("long_strength", &[("strength", 9600, 0)]),
    ("strong_strength", &[("strength", 1800, 1)]),
    ("weakness", &[("weakness", 1800, 0)]),
    ("long_weakness", &[("weakness", 4800, 0)]),
    ("luck", &[("luck", 6000, 0)]),
    ("slow_falling", &[("slow_falling", 1800, 0)]),
    ("long_slow_falling", &[("slow_falling", 4800, 0)]),
    ("wind_charged", &[("wind_charged", 3600, 0)]),
    ("weaving", &[("weaving", 3600, 0)]),
    ("oozing", &[("oozing", 3600, 0)]),
    ("infested", &[("infested", 3600, 0)]),
];

const EFFECT_COLORS: [(&str, u32); 40] = [
    ("speed", 3402751),
    ("slowness", 9154528),
    ("haste", 14270531),
    ("mining_fatigue", 4866583),
    ("strength", 16762624),
    ("instant_health", 16262179),
    ("instant_damage", 11101546),
    ("jump_boost", 16646020),
    ("nausea", 5578058),
    ("regeneration", 13458603),
    ("resistance", 9520880),
    ("fire_resistance", 16750848),
    ("water_breathing", 10017472),
    ("invisibility", 16185078),
    ("blindness", 2039587),
    ("night_vision", 12779366),
    ("hunger", 5797459),
    ("weakness", 4738376),
    ("poison", 8889187),
    ("wither", 7561558),
    ("health_boost", 16284963),
    ("absorption", 2445989),
    ("saturation", 16262179),
    ("glowing", 9740385),
    ("levitation", 13565951),
    ("luck", 5882118),
    ("unluck", 12624973),
    ("slow_falling", 15978425),
    ("conduit_power", 1950417),
    ("dolphins_grace", 8954814),
    ("bad_omen", 745784),
    ("hero_of_the_village", 4521796),
    ("darkness", 2696993),
    ("trial_omen", 1484454),
    ("raid_omen", 14565464),
    ("wind_charged", 12438015),
    ("weaving", 7891290),
    ("oozing", 10092451),
    ("infested", 9214860),
    ("breath_of_the_nautilus", 65518),
];

fn path(id: &str) -> &str {
    id.strip_prefix("minecraft:").unwrap_or(id)
}

fn qualified(id: &str) -> String {
    if id.contains(':') {
        id.to_owned()
    } else {
        format!("minecraft:{id}")
    }
}

pub fn effect_color(effect: &str) -> Option<u32> {
    let name = path(effect);
    EFFECT_COLORS
        .iter()
        .find(|(id, _)| *id == name)
        .map(|&(_, color)| color)
}

pub fn instantaneous(effect: &str) -> bool {
    matches!(
        path(effect),
        "instant_health" | "instant_damage" | "saturation"
    )
}

pub fn potion_effects(potion: &str) -> Option<Vec<PotionEffect>> {
    let name = path(potion);
    POTIONS
        .iter()
        .find(|(id, _)| *id == name)
        .map(|(_, effects)| {
            effects
                .iter()
                .map(|&(effect, duration, amplifier)| {
                    PotionEffect::new(&qualified(effect), duration, amplifier)
                })
                .collect()
        })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PotionEffect {
    pub effect: String,
    pub duration: i32,
    pub amplifier: i32,
    pub ambient: bool,
    pub visible: bool,
}

impl PotionEffect {
    pub fn new(effect: &str, duration: i32, amplifier: i32) -> Self {
        Self {
            effect: qualified(effect),
            duration,
            amplifier: amplifier.clamp(0, 255),
            ambient: false,
            visible: true,
        }
    }

    fn from_json(value: &Value) -> Option<Self> {
        let effect = value.get("id")?.as_str()?;
        let number = |key: &str| value.get(key).and_then(Value::as_i64).unwrap_or(0) as i32;
        let flag =
            |key: &str, default: bool| value.get(key).and_then(Value::as_bool).unwrap_or(default);
        Some(Self {
            ambient: flag("ambient", false),
            visible: flag("show_particles", true),
            ..Self::new(effect, number("duration"), number("amplifier"))
        })
    }

    pub fn instantaneous(&self) -> bool {
        instantaneous(&self.effect)
    }

    pub fn infinite(&self) -> bool {
        self.duration == -1
    }

    pub fn map_duration(&self, map: impl FnOnce(i32) -> i32) -> i32 {
        if self.infinite() || self.duration == 0 {
            self.duration
        } else {
            map(self.duration)
        }
    }

    pub fn scaled(&self, scale: f32) -> Self {
        Self {
            duration: self.map_duration(|d| ((d as f32 * scale).floor() as i32).max(1)),
            ..self.clone()
        }
    }

    pub fn ends_within(&self, ticks: i32) -> bool {
        !self.infinite() && self.duration <= ticks
    }

    pub fn mob_effect(&self) -> Option<EffectInstance> {
        let effect = MobEffect::from_id(&self.effect)?;
        let mut instance = EffectInstance::new(effect, self.duration, self.amplifier);
        instance.ambient = self.ambient;
        instance.visible = self.visible;
        Some(instance)
    }

    pub fn of_mob(effect: &EffectInstance) -> Self {
        Self {
            ambient: effect.ambient,
            visible: effect.visible,
            ..Self::new(effect.effect.id(), effect.duration, effect.amplifier)
        }
    }

    pub fn player_effect(&self) -> Option<EffectKind> {
        Some(match path(&self.effect) {
            "hunger" => EffectKind::Hunger,
            "poison" => EffectKind::Poison,
            "regeneration" => EffectKind::Regeneration,
            "absorption" => EffectKind::Absorption,
            "resistance" => EffectKind::Resistance,
            "fire_resistance" => EffectKind::FireResistance,
            "nausea" => EffectKind::Nausea,
            "slowness" => EffectKind::Slowness,
            "weakness" => EffectKind::Weakness,
            "haste" => EffectKind::Haste,
            "mining_fatigue" => EffectKind::MiningFatigue,
            "conduit_power" => EffectKind::ConduitPower,
            "levitation" => EffectKind::Levitation,
            "slow_falling" => EffectKind::SlowFalling,
            _ => return None,
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PotionContents {
    pub potion: Option<String>,
    pub custom_color: Option<u32>,
    pub custom_effects: Vec<PotionEffect>,
}

impl PotionContents {
    pub fn of(potion: &str) -> Self {
        Self {
            potion: Some(qualified(potion)),
            ..Self::default()
        }
    }

    pub fn from_components(components: Option<&Value>) -> Self {
        match components.and_then(|c| c.get(CONTENTS)) {
            Some(Value::String(potion)) => Self::of(potion),
            Some(contents) => Self {
                potion: contents
                    .get("potion")
                    .and_then(Value::as_str)
                    .map(qualified),
                custom_color: contents
                    .get("custom_color")
                    .and_then(Value::as_i64)
                    .map(|c| c as u32 & 0xFFFFFF),
                custom_effects: contents
                    .get("custom_effects")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(PotionEffect::from_json)
                    .collect(),
            },
            None => Self::default(),
        }
    }

    pub fn is(&self, potion: &str) -> bool {
        self.potion.as_deref().map(path) == Some(path(potion))
    }

    pub fn is_water(&self) -> bool {
        self.is(WATER)
    }

    fn potion_effects(&self) -> Vec<PotionEffect> {
        self.potion
            .as_deref()
            .and_then(potion_effects)
            .into_iter()
            .flatten()
            .collect()
    }

    pub fn all_effects(&self) -> Vec<PotionEffect> {
        let mut effects = self.potion_effects();
        effects.extend(self.custom_effects.iter().cloned());
        effects
    }

    pub fn scaled_effects(&self, scale: f32) -> Vec<PotionEffect> {
        self.all_effects()
            .iter()
            .map(|effect| effect.scaled(scale))
            .collect()
    }

    pub fn has_effects(&self) -> bool {
        !self.custom_effects.is_empty() || !self.potion_effects().is_empty()
    }

    pub fn instant_potion(&self) -> bool {
        self.potion_effects()
            .iter()
            .any(PotionEffect::instantaneous)
    }

    pub fn color(&self) -> u32 {
        if let Some(color) = self.custom_color {
            return color;
        }
        let (mut r, mut g, mut b, mut weight) = (0, 0, 0, 0);
        for effect in self.all_effects().iter().filter(|e| e.visible) {
            let Some(color) = effect_color(&effect.effect) else {
                continue;
            };
            let level = effect.amplifier + 1;
            r += level * ((color >> 16) & 255) as i32;
            g += level * ((color >> 8) & 255) as i32;
            b += level * (color & 255) as i32;
            weight += level;
        }
        if weight == 0 {
            DEFAULT_COLOR
        } else {
            ((r / weight) as u32) << 16 | ((g / weight) as u32) << 8 | (b / weight) as u32
        }
    }
}

pub fn duration_scale(item: &str, components: Option<&Value>) -> f32 {
    if let Some(scale) = components
        .and_then(|c| c.get(DURATION_SCALE))
        .and_then(Value::as_f64)
    {
        return scale as f32;
    }
    match path(item) {
        "lingering_potion" => 0.25,
        "tipped_arrow" => 0.125,
        _ => 1.0,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlayerPotion {
    Effect(PotionEffect),
    Heal(f32),
    Harm { amount: f32, by: Option<u64> },
    Extinguish,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerPotionHit {
    pub player: u64,
    pub potion: PlayerPotion,
    pub at: glam::DVec3,
}

pub fn instant_amount(effect: &PotionEffect, inverted: bool, scale: f64) -> Option<PlayerPotion> {
    let harm = path(&effect.effect) == "instant_damage";
    if !matches!(path(&effect.effect), "instant_health" | "instant_damage") {
        return None;
    }
    let shift = |base: i32| f64::from(base.wrapping_shl(effect.amplifier as u32));
    Some(if harm == inverted {
        PlayerPotion::Heal((scale * shift(4) + 0.5) as i32 as f32)
    } else {
        PlayerPotion::Harm {
            amount: (scale * shift(6) + 0.5) as i32 as f32,
            by: None,
        }
    })
}

pub fn drink_hits(contents: &PotionContents, duration_scale: f32) -> Vec<PlayerPotion> {
    contents
        .scaled_effects(duration_scale)
        .into_iter()
        .filter_map(|effect| {
            if effect.instantaneous() {
                instant_amount(&effect, false, 1.0)
            } else {
                Some(PlayerPotion::Effect(effect))
            }
        })
        .collect()
}

pub fn apply_to_player(status: &mut SurvivalStatus, potion: &PlayerPotion) -> Option<f32> {
    match potion {
        PlayerPotion::Effect(effect) => {
            if let Some(kind) = effect.player_effect() {
                status.add_effect(
                    kind,
                    effect.duration.max(0) as u32,
                    effect.amplifier.clamp(0, 255) as u8,
                );
            }
            None
        }
        PlayerPotion::Heal(amount) => {
            status.heal(*amount);
            None
        }
        PlayerPotion::Harm { amount, .. } => Some(*amount),
        PlayerPotion::Extinguish => {
            status.fire_ticks = 0;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn strong_poison_is_432_ticks_level_2() {
        assert_eq!(
            potion_effects("minecraft:strong_poison"),
            Some(vec![PotionEffect::new("minecraft:poison", 432, 1)])
        );
        assert_eq!(potion_effects("water"), Some(Vec::new()));
        assert_eq!(potion_effects("minecraft:nonsense"), None);
        let turtle = potion_effects("strong_turtle_master").unwrap();
        assert_eq!(
            turtle,
            vec![
                PotionEffect::new("slowness", 400, 5),
                PotionEffect::new("resistance", 400, 3)
            ]
        );
    }

    #[test]
    fn contents_read_the_potion_its_colour_and_custom_effects() {
        let poison = PotionContents::from_components(Some(
            &json!({"minecraft:potion_contents": {"potion": "minecraft:poison"}}),
        ));
        assert!(poison.has_effects() && !poison.instant_potion());
        assert_eq!(poison.color(), 8889187);
        let custom = PotionContents::from_components(Some(&json!({"minecraft:potion_contents": {
            "custom_color": 0x123456,
            "custom_effects": [{"id": "minecraft:speed", "amplifier": 1, "duration": 200}]
        }})));
        assert_eq!(custom.color(), 0x123456);
        assert_eq!(
            custom.all_effects(),
            vec![PotionEffect::new("minecraft:speed", 200, 1)]
        );
        let short = PotionContents::from_components(Some(
            &json!({"minecraft:potion_contents": "minecraft:strong_healing"}),
        ));
        assert!(short.instant_potion());
        let water = PotionContents::from_components(Some(
            &json!({"minecraft:potion_contents": {"potion": "minecraft:water"}}),
        ));
        assert!(water.is_water() && !water.has_effects());
        assert_eq!(water.color(), DEFAULT_COLOR);
        assert_eq!(PotionContents::from_components(None).color(), DEFAULT_COLOR);
    }

    #[test]
    fn colours_mix_by_level() {
        let turtle = PotionContents::of("turtle_master");
        let (s, r) = (9154528u32, 9520880u32);
        let mix = |shift: u32| (((s >> shift) & 255) * 4 + ((r >> shift) & 255) * 3) / 7;
        assert_eq!(turtle.color(), mix(16) << 16 | mix(8) << 8 | mix(0));
    }

    #[test]
    fn durations_scale_down_but_not_to_nothing() {
        let poison = PotionEffect::new("poison", 900, 0);
        assert_eq!(poison.scaled(0.25).duration, 225);
        assert_eq!(PotionEffect::new("poison", 3, 0).scaled(0.25).duration, 1);
        assert_eq!(PotionEffect::new("poison", -1, 0).scaled(0.25).duration, -1);
        assert_eq!(duration_scale("minecraft:lingering_potion", None), 0.25);
        assert_eq!(duration_scale("minecraft:splash_potion", None), 1.0);
        assert_eq!(
            duration_scale(
                "minecraft:splash_potion",
                Some(&json!({"minecraft:potion_duration_scale": 0.5}))
            ),
            0.5
        );
        assert!(poison.scaled(0.25).ends_within(225) && !poison.ends_within(20));
    }

    #[test]
    fn effects_cross_to_mobs_and_players_where_they_exist() {
        let poison = PotionEffect::new("minecraft:poison", 100, 1);
        let mob = poison.mob_effect().unwrap();
        assert_eq!(
            (mob.effect, mob.duration, mob.amplifier),
            (MobEffect::Poison, 100, 1)
        );
        assert_eq!(PotionEffect::of_mob(&mob), poison);
        assert_eq!(poison.player_effect(), Some(EffectKind::Poison));
        assert_eq!(PotionEffect::new("strength", 100, 0).mob_effect(), None);
        assert_eq!(PotionEffect::new("speed", 100, 0).player_effect(), None);
        assert_eq!(
            PotionEffect::new("minecraft:haste", 100, 0).player_effect(),
            Some(EffectKind::Haste)
        );
        assert_eq!(
            PotionEffect::new("minecraft:mining_fatigue", 100, 0).player_effect(),
            Some(EffectKind::MiningFatigue)
        );
        assert_eq!(
            PotionEffect::new("minecraft:conduit_power", 100, 0).player_effect(),
            Some(EffectKind::ConduitPower)
        );
    }

    #[test]
    fn instant_potions_heal_four_and_harm_six_a_level_scaled() {
        let heal = PotionEffect::new("instant_health", 1, 1);
        assert_eq!(
            instant_amount(&heal, false, 1.0),
            Some(PlayerPotion::Heal(8.0))
        );
        assert_eq!(
            instant_amount(&heal, true, 0.5),
            Some(PlayerPotion::Harm {
                amount: 6.0,
                by: None
            })
        );
        let harm = PotionEffect::new("instant_damage", 1, 0);
        assert_eq!(
            instant_amount(&harm, false, 0.75),
            Some(PlayerPotion::Harm {
                amount: 5.0,
                by: None
            })
        );
        assert_eq!(
            instant_amount(&PotionEffect::new("poison", 1, 0), false, 1.0),
            None
        );
    }

    #[test]
    fn a_drunk_potion_gives_its_effects_whole() {
        assert_eq!(
            drink_hits(&PotionContents::of("long_poison"), 1.0),
            vec![PlayerPotion::Effect(PotionEffect::new("poison", 1800, 0))]
        );
        assert_eq!(
            drink_hits(&PotionContents::of("strong_harming"), 1.0),
            vec![PlayerPotion::Harm {
                amount: 12.0,
                by: None
            }]
        );
        assert_eq!(
            drink_hits(&PotionContents::of("healing"), 1.0),
            vec![PlayerPotion::Heal(4.0)]
        );
        assert!(drink_hits(&PotionContents::of("water"), 1.0).is_empty());
    }

    #[test]
    fn a_player_takes_what_it_can_hold() {
        let mut status = SurvivalStatus::default();
        status.health = 10.0;
        status.fire_ticks = 80;
        let poison = PlayerPotion::Effect(PotionEffect::new("poison", 200, 1));
        assert_eq!(apply_to_player(&mut status, &poison), None);
        assert_eq!(
            status
                .effect(EffectKind::Poison)
                .map(|e| (e.duration, e.amplifier)),
            Some((200, 1))
        );
        apply_to_player(&mut status, &PlayerPotion::Heal(4.0));
        assert_eq!(status.health, 14.0);
        let harm = PlayerPotion::Harm {
            amount: 6.0,
            by: Some(3),
        };
        assert_eq!(apply_to_player(&mut status, &harm), Some(6.0));
        apply_to_player(&mut status, &PlayerPotion::Extinguish);
        assert_eq!(status.fire_ticks, 0);
    }
}
