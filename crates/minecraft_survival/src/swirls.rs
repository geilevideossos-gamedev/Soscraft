use glam::DVec3;
use minecraft_terrain::spell_particles::SwirlView;
use minecraftoss_player::survival::{EffectKind, SurvivalStatus};

use crate::player_nbt::EFFECTS;

const PLAYER_RANGE: i32 = 32;
const WIDTH: f32 = 0.6;
const HEIGHT: f32 = 1.8;
const CROUCHING_HEIGHT: f32 = 1.5;

pub fn player_swirl(status: &SurvivalStatus, feet: [f64; 3], crouching: bool) -> Option<SwirlView> {
    let height = if crouching { CROUCHING_HEIGHT } else { HEIGHT };
    let effects = EFFECTS
        .iter()
        .filter(|(kind, _)| status.effect(*kind).is_some())
        .map(|(kind, id)| {
            let color = minecraft_terrain::potions::effect_color(id).unwrap_or(0);
            (color, *kind == EffectKind::ConduitPower, true)
        });
    SwirlView::of_effects(
        (DVec3::from_array(feet), WIDTH, height),
        effects,
        false,
        PLAYER_RANGE,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_poisoned_player_gives_off_a_poison_green_swirl() {
        let mut status = SurvivalStatus::default();
        assert_eq!(player_swirl(&status, [0.5, 64.0, 0.5], false), None);
        status.add_effect(EffectKind::Poison, 900, 0);
        let swirl = player_swirl(&status, [0.5, 64.0, 0.5], false).expect("a swirl");
        assert_eq!(swirl.colors, vec![0xFF87_A363]);
        assert!(!swirl.ambient && !swirl.invisible);
        assert_eq!(
            (swirl.feet, swirl.width, swirl.height, swirl.range),
            (DVec3::new(0.5, 64.0, 0.5), 0.6, 1.8, 32)
        );
        assert_eq!(
            player_swirl(&status, [0.5, 64.0, 0.5], true).map(|swirl| swirl.height),
            Some(1.5)
        );
    }

    #[test]
    fn conduit_power_alone_is_an_ambient_swirl() {
        let mut status = SurvivalStatus::default();
        status.add_effect(EffectKind::ConduitPower, 260, 0);
        let swirl = player_swirl(&status, [0.5, 64.0, 0.5], false).expect("a swirl");
        assert_eq!(swirl.colors, vec![0x261D_C2D1]);
        assert!(swirl.ambient);
        status.add_effect(EffectKind::Regeneration, 100, 1);
        let swirl = player_swirl(&status, [0.5, 64.0, 0.5], false).expect("a swirl");
        assert_eq!(swirl.colors, vec![0xFFCD_5CAB, 0x261D_C2D1]);
        assert!(!swirl.ambient);
    }
}
