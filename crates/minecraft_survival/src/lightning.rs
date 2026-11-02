use crate::death::DamageCause;
use minecraft_terrain::lightning::PlayerStrike;
use minecraft_terrain::protection::Protection;
use minecraftoss_entities::inside_blocks::Hazard;

pub const CAUSE: DamageCause = DamageCause::Hazard(Hazard::LightningBolt);
const IGNITE_TICKS: i32 = 160;

pub fn hit(struck: &PlayerStrike) -> crate::entities::Hit {
    (
        minecraft_terrain::lightning::DAMAGE,
        Some(struck.at),
        None,
        CAUSE,
        Default::default(),
        None,
    )
}

pub fn burn(fire_ticks: &mut i32, protection: &Protection, cause: DamageCause) {
    if cause != CAUSE {
        return;
    }
    *fire_ticks += 1;
    if *fire_ticks == 0 {
        *fire_ticks = protection.burning_ticks(IGNITE_TICKS).max(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::survival::Survival;
    use minecraftoss_player::survival::{Armor, SurvivalStatus};

    fn resting(fire_ticks: i32, health: f32) -> Survival {
        let mut survival = Survival::default();
        let mut status = SurvivalStatus::default();
        status.health = health;
        survival.restore(status, fire_ticks, 0, 0);
        survival
    }

    fn strike(survival: &mut Survival) -> bool {
        let (damage, from, toward, cause, push, effect) = hit(&PlayerStrike {
            player: 0,
            at: [0.5, 64.0, 0.5],
        });
        let blocked = crate::offhand::blocked_damage(0.0, toward, damage);
        assert_eq!(blocked, 0.0, "the shield never blocks lightning");
        assert_eq!(push.landing(true, false, 0.0), None, "no knockback");
        survival.struck(damage, blocked, Armor::default(), from, cause, effect)
    }

    #[test]
    fn lightning_ignores_the_shield_and_knockback_and_lights_no_resting_player() {
        let mut resting = resting(-20, 20.0);
        assert!(strike(&mut resting));
        assert_eq!(resting.status.health, 15.0);
        assert_eq!(
            resting.fire_ticks(),
            -19,
            "the hit itself lights no resting player"
        );
        let mut cooling = self::resting(-1, 20.0);
        strike(&mut cooling);
        assert_eq!(
            cooling.fire_ticks(),
            160,
            "-1 becomes 0, then eight seconds"
        );
        let mut burning = self::resting(100, 20.0);
        strike(&mut burning);
        assert_eq!(burning.fire_ticks(), 101);
        let mut other = self::resting(-1, 20.0);
        other.struck(3.0, 0.0, Armor::default(), None, DamageCause::Generic, None);
        assert_eq!(other.fire_ticks(), -1, "only lightning bumps the fire");
    }

    #[test]
    fn a_lightning_death_reads_struck_by_lightning() {
        let mut survival = resting(-20, 2.0);
        strike(&mut survival);
        assert_eq!(survival.status.health, 0.0);
        let language = [("death.attack.lightningBolt", "%1$s was struck by lightning")]
            .into_iter()
            .map(|(key, text)| (key.to_owned(), text.to_owned()))
            .collect();
        let line = survival.death_line(&language, |_| true, |_| None);
        assert_eq!(line.message, "%1$s was struck by lightning");
        assert_eq!(CAUSE.damage_type(), "minecraft:lightning_bolt");
    }

    #[test]
    fn summon_lists_the_lightning_bolt() {
        let packs = minecraft_terrain::pack::PackStack::open(Vec::new()).unwrap();
        let entities = crate::commands::registries(&packs).entities;
        assert!(
            entities
                .iter()
                .any(|kind| kind == minecraft_terrain::lightning::BOLT)
        );
        assert!(
            entities.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted, each once"
        );
    }
}
