use minecraft_commands::text::RED;
use minecraft_commands::{Clientbound, Text};
use minecraft_net::Dimension;

use crate::player_nbt::Place;

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
}
