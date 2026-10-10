use minecraft_commands::text::RED;
use minecraft_commands::{Clientbound, Text};
use minecraft_net::Dimension;
use minecraftoss_core::Registries;
use minecraftoss_core::ident::Identifier;

use crate::player_nbt::Place;

pub const DIMENSIONS: [Dimension; 3] = [Dimension::Overworld, Dimension::Nether, Dimension::End];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadingReason {
    NetherPortal,
    EndPortal,
    #[default]
    Other,
}

pub fn loading_reason(dead: bool, from: Dimension, to: Dimension) -> LoadingReason {
    if dead {
        LoadingReason::Other
    } else if from == Dimension::Nether || to == Dimension::Nether {
        LoadingReason::NetherPortal
    } else if from == Dimension::End || to == Dimension::End {
        LoadingReason::EndPortal
    } else {
        LoadingReason::Other
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Switch {
    pub from: Dimension,
    pub to: Dimension,
    pub arrival: Option<Place>,
    pub reason: LoadingReason,
    pub respawn: bool,
    pub keep_charge: bool,
    pub left: bool,
}

impl Switch {
    pub fn new(
        from: Dimension,
        to: Dimension,
        arrival: Option<Place>,
        reason: LoadingReason,
    ) -> Self {
        Self {
            from,
            to,
            arrival: arrival.filter(|place| place.dimension == to),
            reason,
            respawn: false,
            keep_charge: false,
            left: false,
        }
    }
}

pub fn failed_message(dimension: Dimension, error: &str) -> Clientbound {
    Clientbound::System {
        text: Text::literal(format!(
            "{} could not be loaded: {error}",
            dimension.dimension_type()
        ))
        .color(RED),
        overlay: false,
    }
}

pub fn dimension_named(id: &str) -> Option<Dimension> {
    let id = Identifier::parse(id).ok()?;
    DIMENSIONS
        .into_iter()
        .find(|dimension| dimension.dimension_type() == id.as_str())
}

pub fn coordinate_scale(registries: &Registries, dimension: Dimension) -> Result<f64, String> {
    let id = Identifier::parse(dimension.dimension_type())?;
    registries
        .datapack
        .read_json("dimension_type", &id)?
        .get("coordinate_scale")
        .and_then(serde_json::Value::as_f64)
        .ok_or_else(|| format!("{} has no coordinate_scale", dimension.dimension_type()))
}

pub fn teleport_scale(
    registries: &Registries,
    from: Dimension,
    to: Dimension,
) -> Result<f64, String> {
    Ok(coordinate_scale(registries, from)? / coordinate_scale(registries, to)?)
}

pub fn scaled(feet: [f64; 3], scale: f64) -> [f64; 3] {
    [feet[0] * scale, feet[1], feet[2] * scale]
}

pub fn air_support_unavailable() -> Clientbound {
    Clientbound::System {
        text: Text::literal("Air support is unavailable under this dimension's ceiling"),
        overlay: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [Dimension; 3] = [Dimension::Overworld, Dimension::Nether, Dimension::End];

    #[test]
    fn the_loading_reason_follows_determine_level_loading_reason() {
        use Dimension::{End, Nether, Overworld};
        use LoadingReason::{EndPortal, NetherPortal, Other};
        let expected = [
            (Overworld, Overworld, Other),
            (Overworld, Nether, NetherPortal),
            (Overworld, End, EndPortal),
            (Nether, Overworld, NetherPortal),
            (Nether, Nether, NetherPortal),
            (Nether, End, NetherPortal),
            (End, Overworld, EndPortal),
            (End, Nether, NetherPortal),
            (End, End, EndPortal),
        ];
        for (from, to, reason) in expected {
            assert_eq!(
                loading_reason(false, from, to),
                reason,
                "{from:?} to {to:?}"
            );
        }
        for from in ALL {
            for to in ALL {
                assert_eq!(
                    loading_reason(true, from, to),
                    Other,
                    "dead, {from:?} to {to:?}"
                );
            }
        }
    }

    #[test]
    fn a_switch_keeps_only_an_arrival_in_its_target_dimension() {
        let roof = Place::standing(Dimension::Nether, [10.5, 128.0, -20.5], 90.0, 0.0);
        let switch = Switch::new(
            Dimension::Overworld,
            Dimension::Nether,
            Some(roof),
            LoadingReason::NetherPortal,
        );
        assert_eq!(switch.arrival, Some(roof));
        assert!(!switch.respawn && !switch.left);
        let elsewhere = Switch::new(
            Dimension::Overworld,
            Dimension::End,
            Some(roof),
            LoadingReason::EndPortal,
        );
        assert_eq!(elsewhere.arrival, None);
        assert_eq!(LoadingReason::default(), LoadingReason::Other);
    }

    #[test]
    fn a_level_that_failed_to_load_says_so_in_red() {
        let Clientbound::System { text, overlay } =
            failed_message(Dimension::End, "the level loader stopped")
        else {
            panic!("a system line");
        };
        assert!(!overlay);
        assert_eq!(text.color, Some(RED));
        let line = text.resolve(&|_| None).text;
        assert_eq!(
            line,
            "minecraft:the_end could not be loaded: the level loader stopped"
        );
    }

    #[test]
    fn a_dimension_is_named_by_its_id_with_or_without_the_namespace() {
        assert_eq!(
            dimension_named("minecraft:the_nether"),
            Some(Dimension::Nether)
        );
        assert_eq!(dimension_named("the_end"), Some(Dimension::End));
        assert_eq!(dimension_named("overworld"), Some(Dimension::Overworld));
        assert_eq!(dimension_named("nether"), None, "not a dimension id");
        assert_eq!(dimension_named("example:the_nether"), None);
        assert_eq!(
            dimension_named("Minecraft:The_End"),
            None,
            "ids are lower case"
        );
        for dimension in DIMENSIONS {
            assert_eq!(dimension_named(dimension.dimension_type()), Some(dimension));
        }
    }

    #[test]
    fn the_teleport_scale_divides_the_coordinate_scales() {
        let Some(registries) = minecraftoss_core::registries::DataPaths::discover()
            .ok()
            .and_then(|paths| Registries::load(&paths).ok())
        else {
            return;
        };
        let scale = |from, to| teleport_scale(&registries, from, to).unwrap();
        assert_eq!(scale(Dimension::Overworld, Dimension::Nether), 1.0 / 8.0);
        assert_eq!(scale(Dimension::Nether, Dimension::Overworld), 8.0);
        assert_eq!(scale(Dimension::Overworld, Dimension::End), 1.0);
        assert_eq!(scale(Dimension::End, Dimension::Nether), 1.0 / 8.0);
        assert_eq!(scale(Dimension::Nether, Dimension::Nether), 1.0);
        assert_eq!(
            scaled(
                [800.0, 70.0, -1600.0],
                scale(Dimension::Overworld, Dimension::Nether)
            ),
            [100.0, 70.0, -200.0]
        );
    }

    #[test]
    fn refused_air_support_says_why_in_the_chat() {
        let Clientbound::System { text, overlay } = air_support_unavailable() else {
            panic!("a system line");
        };
        assert!(!overlay, "an overlay line would not show");
        assert_eq!(
            text.resolve(&|_| None).text,
            "Air support is unavailable under this dimension's ceiling"
        );
    }
}
