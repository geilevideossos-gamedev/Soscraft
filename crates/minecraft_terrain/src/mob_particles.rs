use glam::DVec3;
use minecraftoss_entities::feline::{DISTRUSTING_EVENT, TRUSTING_EVENT};
use minecraftoss_entities::world::EntityWorld;

const VILLAGER: &str = "minecraft:villager";
const OCELOT: &str = "minecraft:ocelot";
const TAMED: u8 = 7;
const NOT_TAMED: u8 = 6;
const IN_LOVE: u8 = 18;
pub const SPAWN_ANIM: u8 = 20;
const POOF_PARTICLES: u8 = 20;
const VILLAGER_HEARTS: u8 = 12;
const VILLAGER_ANGRY: u8 = 13;
const VILLAGER_HAPPY: u8 = 14;
const VILLAGER_SWEATING: u8 = 42;
const ANIMAL_PARTICLES: u8 = 7;
const ANIMAL_LIFT: f64 = 0.5;
const VILLAGER_PARTICLES: u8 = 5;
const VILLAGER_LIFT: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PuffKind {
    Heart,
    Smoke,
    Poof,
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
    let villager = |kind| Some((kind, VILLAGER_PARTICLES, VILLAGER_LIFT));
    let animal = |kind| Some((kind, ANIMAL_PARTICLES, ANIMAL_LIFT));
    match (entity_type, event) {
        (_, SPAWN_ANIM) => Some((PuffKind::Poof, POOF_PARTICLES, 0.0)),
        (VILLAGER, VILLAGER_HEARTS) => villager(PuffKind::Heart),
        (VILLAGER, VILLAGER_ANGRY) => villager(PuffKind::AngryVillager),
        (VILLAGER, VILLAGER_HAPPY) => villager(PuffKind::HappyVillager),
        (VILLAGER, VILLAGER_SWEATING) => villager(PuffKind::Splash),
        (VILLAGER, _) => None,
        (OCELOT, TRUSTING_EVENT) => animal(PuffKind::Heart),
        (OCELOT, DISTRUSTING_EVENT) => animal(PuffKind::Smoke),
        (OCELOT, TAMED | NOT_TAMED) => None,
        (_, TAMED | IN_LOVE) => animal(PuffKind::Heart),
        (_, NOT_TAMED) => animal(PuffKind::Smoke),
        _ => None,
    }
}

pub fn mob_puffs(world: &EntityWorld, events: &[(u64, u8)]) -> Vec<MobPuff> {
    events
        .iter()
        .filter_map(|&(id, event)| {
            let (entity_type, body) = match (world.entity_type(id), world.body_of(id)) {
                (Some(entity_type), Some(body)) => (entity_type, body),
                _ => world.departed(id)?,
            };
            let (kind, count, lift) = puff_for(entity_type, event)?;
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
    use minecraftoss_player::{Block, Pos, World};

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
    fn villager_events_give_five_particles_a_block_up() {
        for (event, kind) in [
            (12, PuffKind::Heart),
            (13, PuffKind::AngryVillager),
            (14, PuffKind::HappyVillager),
            (42, PuffKind::Splash),
        ] {
            assert_eq!(puff_for("minecraft:villager", event), Some((kind, 5, 1.0)));
        }
        assert_eq!(puff_for("minecraft:wolf", 12), None);
        assert_eq!(puff_for("minecraft:wandering_trader", 14), None);
        let mut world = EntityWorld::default();
        let villager = world.spawn_villager(
            minecraftoss_entities::villager::Villager::new(DVec3::new(0.5, 64.0, 0.5)),
            true,
        );
        let puffs = mob_puffs(&world, &[(villager, 42), (villager, 18)]);
        assert_eq!(puffs.len(), 1);
        assert_eq!((puffs[0].kind, puffs[0].count), (PuffKind::Splash, 5));
        assert_eq!((puffs[0].lift, puffs[0].range), (1.0, 10));
    }

    #[test]
    fn a_villager_in_love_and_a_ravager_s_roar_draw_nothing_here() {
        assert_eq!(puff_for("minecraft:villager", 18), None);
        assert_eq!(puff_for("minecraft:villager", 7), None);
        assert_eq!(puff_for("minecraft:ravager", 39), None);
        assert_eq!(puff_for("minecraft:ravager", 69), None);
    }

    #[test]
    fn a_spawn_animation_is_twenty_poofs_on_any_mob() {
        for mob in [
            "minecraft:silverfish",
            "minecraft:villager",
            "minecraft:ocelot",
            "minecraft:zombie",
        ] {
            assert_eq!(
                puff_for(mob, SPAWN_ANIM),
                Some((PuffKind::Poof, 20, 0.0)),
                "{mob}"
            );
        }
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

    struct StoneRing;

    impl World for StoneRing {
        fn block(&self, (x, y, z): Pos) -> Option<Block> {
            if y < 64 {
                return Some(Block::new("minecraft:dirt"));
            }
            let wall = (64..=65).contains(&y) && x.abs() + z.abs() == 1;
            wall.then(|| Block::new("minecraft:stone_bricks"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    #[test]
    fn a_silverfish_that_merged_into_stone_still_poofs() {
        let mut world = EntityWorld::default();
        world.set_mob_griefing(true);
        let at = minecraftoss_core::BlockPos::new(0, 64, 0);
        let tag = crate::infested::silverfish_tag(at, [1, 2, 3, 4]);
        let id = crate::server_mobs::spawn_saved_all(&mut world, &tag)[0].0;
        let mut puffs = Vec::new();
        for _ in 0..2400 {
            world.tick_with_players(&mut StoneRing, &[]);
            let events = world.take_entity_events();
            puffs.extend(mob_puffs(&world, &events));
            if world.entity_type(id).is_none() {
                break;
            }
        }
        assert_eq!(world.entity_type(id), None, "it went into the wall");
        let (_, body) = world.departed(id).expect("it left this tick");
        assert_eq!(
            puffs,
            vec![MobPuff {
                kind: PuffKind::Poof,
                count: 20,
                feet: body.position,
                width: 0.4,
                height: 0.3,
                lift: 0.0,
                range: 8,
            }]
        );
        assert!((body.position.x - 0.5).abs() < 0.5 && (body.position.z - 0.5).abs() < 0.5);
    }
}
