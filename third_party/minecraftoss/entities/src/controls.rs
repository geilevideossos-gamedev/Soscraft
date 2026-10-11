#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerControls {
    pub id: u64,
    pub yaw: f32,
    pub pitch: f32,
    pub main_hand: Option<String>,
    pub offhand: Option<String>,
    pub sneaking: bool,
}

impl PlayerControls {
    pub fn holds(&self, accepts: impl Fn(&str) -> bool) -> bool {
        self.main_hand.as_deref().is_some_and(&accepts)
            || self.offhand.as_deref().is_some_and(&accepts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn either_hand_counts() {
        let fungus = |item: &str| item == "minecraft:warped_fungus";
        let mut controls = PlayerControls {
            id: 3,
            offhand: Some("minecraft:warped_fungus".to_owned()),
            ..PlayerControls::default()
        };
        assert!(controls.holds(fungus));
        controls.offhand = None;
        assert!(!controls.holds(fungus));
        controls.main_hand = Some("minecraft:warped_fungus".to_owned());
        assert!(controls.holds(fungus));
    }
}
