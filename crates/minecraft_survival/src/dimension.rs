use minecraft_net::Dimension;
use minecraft_terrain::environment::dimension_value;
use minecraftoss_core::Registries;
use minecraftoss_core::environment::{EnvironmentSystem, Value};
use minecraftoss_core::ident::Identifier;
use serde_json::Value as Json;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BedRuleTest {
    Always,
    WhenDark,
    Never,
}

impl BedRuleTest {
    fn parse(json: Option<&Json>) -> Option<Self> {
        Some(match json?.as_str()? {
            "always" => Self::Always,
            "when_dark" => Self::WhenDark,
            "never" => Self::Never,
            _ => return None,
        })
    }

    pub fn holds(self, dark: bool) -> bool {
        match self {
            Self::Always => true,
            Self::WhenDark => dark,
            Self::Never => false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BedRule {
    pub can_sleep: BedRuleTest,
    pub can_set_spawn: BedRuleTest,
    pub destroy_on_use: bool,
    pub destroy_on_leave: bool,
    pub error_message: Option<String>,
}

impl BedRule {
    pub fn can_sleep_when_dark() -> Self {
        Self {
            can_sleep: BedRuleTest::WhenDark,
            can_set_spawn: BedRuleTest::Always,
            destroy_on_use: false,
            destroy_on_leave: false,
            error_message: Some("block.minecraft.bed.no_sleep".to_owned()),
        }
    }

    fn parse(json: &Json) -> Option<Self> {
        let flag = |key: &str| json.get(key).and_then(Json::as_bool).unwrap_or(false);
        Some(Self {
            can_sleep: BedRuleTest::parse(json.get("can_sleep"))?,
            can_set_spawn: BedRuleTest::parse(json.get("can_set_spawn"))?,
            destroy_on_use: flag("destroy_on_use"),
            destroy_on_leave: flag("destroy_on_leave"),
            error_message: json.get("error_message").and_then(|message| {
                message
                    .get("translate")
                    .or_else(|| message.get("text"))
                    .or(Some(message))
                    .and_then(Json::as_str)
                    .map(str::to_owned)
            }),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DimensionRules {
    pub dimension: Dimension,
    pub coordinate_scale: f64,
    pub min_y: i32,
    pub height: i32,
    pub logical_height: i32,
    pub has_skylight: bool,
    pub has_ceiling: bool,
    pub nether_cardinal_light: bool,
    pub bed_rule: BedRule,
    pub straw_bed_rule: BedRule,
    pub respawn_anchor_works: bool,
    pub water_evaporates: bool,
    pub fast_lava: bool,
    pub nether_portal_spawns_piglin: bool,
}

impl DimensionRules {
    pub fn load(registries: &Registries, dimension: Dimension) -> Result<Self, String> {
        let name = dimension.dimension_type();
        let json = registries
            .datapack
            .read_json("dimension_type", &Identifier::parse(name)?)?;
        let (environment, _) = EnvironmentSystem::for_dimension(registries, name)?;
        Ok(Self::read(dimension, &json, &environment))
    }

    fn read(dimension: Dimension, json: &Json, environment: &EnvironmentSystem) -> Self {
        let int = |key: &str, default: i64| {
            json.get(key).and_then(Json::as_i64).unwrap_or(default) as i32
        };
        let flag = |key: &str| json.get(key).and_then(Json::as_bool).unwrap_or(false);
        let attribute = |name: &str| dimension_value(environment, name);
        let height = int("height", 256);
        let bed_rule = |name: &str| {
            match attribute(name) {
                Value::Json(rule) => BedRule::parse(&rule),
                _ => None,
            }
            .unwrap_or_else(BedRule::can_sleep_when_dark)
        };
        Self {
            dimension,
            coordinate_scale: json
                .get("coordinate_scale")
                .and_then(Json::as_f64)
                .unwrap_or(1.0),
            min_y: int("min_y", 0),
            height,
            logical_height: int("logical_height", i64::from(height)),
            has_skylight: flag("has_skylight"),
            has_ceiling: flag("has_ceiling"),
            nether_cardinal_light: json.get("cardinal_light").and_then(Json::as_str)
                == Some("nether"),
            bed_rule: bed_rule("gameplay/bed_rule"),
            straw_bed_rule: bed_rule("gameplay/straw_bed_rule"),
            respawn_anchor_works: attribute("gameplay/respawn_anchor_works").as_bool(),
            water_evaporates: attribute("gameplay/water_evaporates").as_bool(),
            fast_lava: attribute("gameplay/fast_lava").as_bool(),
            nether_portal_spawns_piglin: attribute("gameplay/nether_portal_spawns_piglin")
                .as_bool(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn registries() -> Option<Registries> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok()
    }

    #[test]
    fn a_dimension_type_gives_its_scale_heights_lighting_and_gameplay_rules() {
        let attributes = json!({
            "minecraft:gameplay/bed_rule": {"can_set_spawn": "never", "can_sleep": "never", "destroy_on_use": true},
            "minecraft:gameplay/fast_lava": true,
            "minecraft:gameplay/respawn_anchor_works": true,
            "minecraft:gameplay/water_evaporates": true
        });
        let environment = EnvironmentSystem::new(&attributes, &[], &[], false).unwrap();
        let json = json!({
            "cardinal_light": "nether",
            "coordinate_scale": 8.0,
            "has_ceiling": true,
            "has_skylight": false,
            "height": 256,
            "logical_height": 128,
            "min_y": 0
        });
        let rules = DimensionRules::read(Dimension::Nether, &json, &environment);
        assert_eq!(rules.coordinate_scale, 8.0);
        assert_eq!(
            (rules.min_y, rules.height, rules.logical_height),
            (0, 256, 128)
        );
        assert!(rules.has_ceiling && !rules.has_skylight && rules.nether_cardinal_light);
        assert!(rules.fast_lava && rules.water_evaporates && rules.respawn_anchor_works);
        assert!(!rules.nether_portal_spawns_piglin);
        assert_eq!(
            rules.bed_rule,
            BedRule {
                can_sleep: BedRuleTest::Never,
                can_set_spawn: BedRuleTest::Never,
                destroy_on_use: true,
                destroy_on_leave: false,
                error_message: None,
            }
        );
    }

    #[test]
    fn a_dimension_without_a_bed_rule_sleeps_when_dark_and_sets_the_spawn() {
        let environment = EnvironmentSystem::new(&json!({}), &[], &[], false).unwrap();
        let rules =
            DimensionRules::read(Dimension::Overworld, &json!({"height": 384}), &environment);
        assert_eq!(rules.bed_rule, BedRule::can_sleep_when_dark());
        assert_eq!(rules.logical_height, 384);
        assert_eq!(rules.coordinate_scale, 1.0);
        assert!(!rules.nether_cardinal_light && !rules.fast_lava);
        assert!(rules.bed_rule.can_set_spawn.holds(false));
        assert!(rules.bed_rule.can_sleep.holds(true) && !rules.bed_rule.can_sleep.holds(false));
        assert!(!BedRuleTest::Never.holds(true));
    }

    #[test]
    fn the_three_vanilla_dimensions_have_their_own_rules() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionRules::load(&registries, Dimension::Overworld).unwrap();
        assert_eq!(overworld.coordinate_scale, 1.0);
        assert_eq!(
            (overworld.min_y, overworld.height, overworld.logical_height),
            (-64, 384, 384)
        );
        assert!(overworld.has_skylight && !overworld.has_ceiling);
        assert!(!overworld.nether_cardinal_light);
        assert_eq!(overworld.bed_rule, BedRule::can_sleep_when_dark());
        assert_eq!(overworld.straw_bed_rule.can_set_spawn, BedRuleTest::Never);
        assert!(overworld.straw_bed_rule.destroy_on_leave && !overworld.straw_bed_rule.destroy_on_use);
        assert!(!overworld.respawn_anchor_works && !overworld.water_evaporates);
        assert!(!overworld.fast_lava && overworld.nether_portal_spawns_piglin);

        let nether = DimensionRules::load(&registries, Dimension::Nether).unwrap();
        assert_eq!(nether.coordinate_scale, 8.0);
        assert_eq!(
            (nether.min_y, nether.height, nether.logical_height),
            (0, 256, 128)
        );
        assert!(!nether.has_skylight && nether.has_ceiling);
        assert!(nether.nether_cardinal_light);
        assert_eq!(nether.bed_rule.can_sleep, BedRuleTest::Never);
        assert_eq!(nether.bed_rule.can_set_spawn, BedRuleTest::Never);
        assert!(nether.bed_rule.destroy_on_use && !nether.bed_rule.destroy_on_leave);
        assert!(nether.straw_bed_rule.destroy_on_use);
        assert_eq!(nether.straw_bed_rule.error_message, None);
        assert!(nether.respawn_anchor_works && nether.water_evaporates);
        assert!(nether.fast_lava && !nether.nether_portal_spawns_piglin);

        let end = DimensionRules::load(&registries, Dimension::End).unwrap();
        assert_eq!(end.coordinate_scale, 1.0);
        assert_eq!((end.min_y, end.height, end.logical_height), (0, 256, 256));
        assert!(end.has_skylight && !end.has_ceiling && !end.nether_cardinal_light);
        assert!(end.bed_rule.destroy_on_use && end.straw_bed_rule.destroy_on_use);
        assert_eq!(end.bed_rule.can_set_spawn, BedRuleTest::Never);
        assert!(!end.respawn_anchor_works && !end.water_evaporates);
        assert!(!end.fast_lava && !end.nether_portal_spawns_piglin);
    }
}
