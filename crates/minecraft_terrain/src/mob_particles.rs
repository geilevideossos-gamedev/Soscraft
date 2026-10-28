use glam::DVec3;
use minecraftoss_entities::feline::{DISTRUSTING_EVENT, TRUSTING_EVENT};
use minecraftoss_entities::world::EntityWorld;

const VILLAGER: &str = "minecraft:villager";
const OCELOT: &str = "minecraft:ocelot";
const TAMED: u8 = 7;
const NOT_TAMED: u8 = 6;
const IN_LOVE: u8 = 18;
const ANIMAL_PARTICLES: u8 = 7;
const ANIMAL_LIFT: f64 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PuffKind {
    Heart,
    Smoke,
    AngryVillager,
    HappyVillager,
    Splash,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MobPuff {
    pub kind: PuffKind,
    pub count: u8,
    pub feet: DVec3,
    pub width: f32,
    pub height: f32,
    pub lift: f64,
    pub range: i32,
}

pub fn puff_for(entity_type: &str, event: u8) -> Option<(PuffKind, u8, f64)> {
    let kind = match (entity_type, event) {
        (VILLAGER, _) => None,
        (OCELOT, TRUSTING_EVENT) => Some(PuffKind::Heart),
        (OCELOT, DISTRUSTING_EVENT) => Some(PuffKind::Smoke),
        (OCELOT, TAMED | NOT_TAMED) => None,
        (_, TAMED | IN_LOVE) => Some(PuffKind::Heart),
        (_, NOT_TAMED) => Some(PuffKind::Smoke),
        _ => None,
    };
    kind.map(|kind| (kind, ANIMAL_PARTICLES, ANIMAL_LIFT))
}

pub fn mob_puffs(world: &EntityWorld, events: &[(u64, u8)]) -> Vec<MobPuff> {
    events
        .iter()
        .filter_map(|&(id, event)| {
            let entity_type = world.entity_type(id)?;
            let (kind, count, lift) = puff_for(entity_type, event)?;
            let body = world.body_of(id)?;
            Some(MobPuff {
                kind,
                count,
                feet: body.position,
                width: body.width,
                height: body.height,
                lift,
                range: crate::mob_tracking::tracking_range(entity_type),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::wolf::Wolf;

    #[test]
    fn taming_shows_seven_hearts_and_a_failed_try_seven_smokes() {
        for mob in [
            "minecraft:horse",
            "minecraft:llama",
            "minecraft:wolf",
            "minecraft:cat",
        ] {
            assert_eq!(puff_for(mob, 7), Some((PuffKind::Heart, 7, 0.5)), "{mob}");
            assert_eq!(puff_for(mob, 6), Some((PuffKind::Smoke, 7, 0.5)), "{mob}");
            assert_eq!(puff_for(mob, 18), Some((PuffKind::Heart, 7, 0.5)), "{mob}");
            assert_eq!(puff_for(mob, 4), None, "{mob}");
        }
    }

    #[test]
    fn an_ocelot_shows_hearts_when_it_trusts_and_smoke_when_it_does_not() {
        assert_eq!(
            puff_for("minecraft:ocelot", 41),
            Some((PuffKind::Heart, 7, 0.5))
        );
        assert_eq!(
            puff_for("minecraft:ocelot", 40),
            Some((PuffKind::Smoke, 7, 0.5))
        );
        assert_eq!(
            puff_for("minecraft:ocelot", 18),
            Some((PuffKind::Heart, 7, 0.5)),
            "an ocelot in love is an animal in love"
        );
        assert_eq!(puff_for("minecraft:ocelot", 7), None);
        assert_eq!(puff_for("minecraft:ocelot", 6), None);
    }

    #[test]
    fn a_villager_in_love_and_a_ravager_s_roar_draw_nothing_here() {
        assert_eq!(puff_for("minecraft:villager", 18), None);
        assert_eq!(puff_for("minecraft:villager", 7), None);
        assert_eq!(puff_for("minecraft:ravager", 39), None);
        assert_eq!(puff_for("minecraft:ravager", 69), None);
    }

    #[test]
    fn an_event_of_a_mob_that_is_gone_is_dropped() {
        let mut world = EntityWorld::default();
        let wolf = world.spawn_wolf(Wolf::new(DVec3::new(0.5, 64.0, 0.5)), true);
        assert!(mob_puffs(&world, &[(wolf + 100, 7)]).is_empty());
        assert!(mob_puffs(&world, &[(wolf, 4)]).is_empty());
        assert_eq!(mob_puffs(&world, &[(wolf + 100, 7), (wolf, 6)]).len(), 1);
    }

    #[test]
    fn a_puff_sits_on_the_mob_s_body() {
        let mut world = EntityWorld::default();
        let wolf = world.spawn_wolf(Wolf::new(DVec3::new(3.5, 64.0, -2.5)), true);
        let body = world.body_of(wolf).unwrap().clone();
        assert_eq!(
            mob_puffs(&world, &[(wolf, 7)]),
            vec![MobPuff {
                kind: PuffKind::Heart,
                count: 7,
                feet: body.position,
                width: body.width,
                height: body.height,
                lift: 0.5,
                range: 10,
            }]
        );
        assert_eq!(body.position, DVec3::new(3.5, 64.0, -2.5));
        assert!(body.width > 0.5 && body.height > 0.8);
    }
}
