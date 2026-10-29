use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::look::BodyRotation;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::trial::breezes::Breeze;
use minecraftoss_entities::world::families::trial::breezes::MAX_HEAD_Y_ROT;

pub const WHIRL_SOUND: &str = "minecraft:entity.breeze.whirl";
pub const WHIRL_MAX_TICKS: u32 = 80;

impl TerrainFamily for Breeze {
    const TRACKING_RANGE: i32 = 10;
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = 0.5;
    const MAX_HEAD_Y_ROT: f32 = MAX_HEAD_Y_ROT;

    fn read(mob: &mut FamilyMob<Self>, _tag: &Tag) {
        mob.state.body_rotation = BodyRotation::new(mob.core.yaw);
    }

    fn draw(
        _out: &mut FamilyDraw<'_>,
        _mobs: &[FamilyMob<Self>],
        _poses: &ClientMobs,
        _atlas: &Atlas,
        _light: &SkyLight,
        _partial: f32,
    ) {
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            let tick = fx.seen.entry(mob.core.id).or_insert(0);
            *tick = if *tick == 0 {
                1 + fx.random.next_int(WHIRL_MAX_TICKS) as i32
            } else {
                *tick - 1
            };
            if *tick == 0 {
                let pitch = 0.7 + 0.4 * fx.random.next_float();
                let volume = 0.8 + 0.2 * fx.random.next_float();
                fx.sounds
                    .push((WHIRL_SOUND.to_owned(), shown.position, volume, pitch));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_tracking::{MobEvent, Trackers, Watcher};
    use glam::DVec3;
    use minecraftoss_entities::world::EntityWorld;
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    #[test]
    fn a_breeze_saves_and_loads_with_its_health_and_turn() {
        let tag = parse(
            "{id:\"minecraft:breeze\",Pos:[1.5d,64.0d,2.5d],UUID:[I;4,5,6,40],Rotation:[90.0f,0.0f],Health:12.0f,PersistenceRequired:1b,CustomName:\"Gust\"}",
        );
        let mut world = EntityWorld::default();
        let id = crate::server_mobs::spawn_saved(&mut world, &tag).expect("a breeze loads");
        assert_eq!(world.entity_type(id), Some("minecraft:breeze"));
        let mob = &world.family::<Breeze>()[0];
        assert_eq!(
            (mob.core.body.position, mob.core.yaw, mob.core.look.head_yaw),
            (DVec3::new(1.5, 64.0, 2.5), 90.0, 90.0)
        );
        assert_eq!(
            (mob.core.health, mob.state.body_rotation.body_yaw),
            (12.0, 90.0)
        );
        assert!(mob.core.persistent);
        world.family_core_mut(id).unwrap().1.health = 7.0;
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = crate::server_mobs::mob_tags(&world, |_| true, &originals);
        let (_, saved) = saved.into_iter().find(|(saved, _)| *saved == id).unwrap();
        assert_eq!(saved.get("Health"), Some(&Tag::Float(7.0)));
        assert_eq!(saved.get("CustomName").and_then(Tag::as_str), Some("Gust"));
        let mut again = EntityWorld::default();
        let back = crate::server_mobs::spawn_saved(&mut again, &saved).expect("it loads again");
        assert_eq!(again.family_core(back).unwrap().1.health, 7.0);
        assert_eq!(again.uuid_of(back), world.uuid_of(id));
    }

    #[test]
    fn a_friend_sees_a_breeze_from_its_look_tag() {
        assert_eq!(
            (
                crate::mob_tracking::tracking_range("minecraft:breeze"),
                crate::mob_tracking::tracking_range("breeze")
            ),
            (10, 10)
        );
        let mut host = EntityWorld::default();
        let tag =
            parse("{id:\"minecraft:breeze\",Pos:[3.5d,64.0d,0.5d],Rotation:[45.0f,0.0f],NoAI:1b}");
        let id = crate::server_mobs::spawn_saved(&mut host, &tag).unwrap();
        let mut trackers = Trackers::default();
        trackers.tick();
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let watcher = Watcher {
            id: 3,
            feet: [0.5, 64.0, 0.5],
            view_distance: 8,
        };
        let tracked = trackers.update(&host, &originals, &[watcher]);
        let mut friend = EntityWorld::default();
        let mut added = Vec::new();
        for event in tracked.events_of(3) {
            let MobEvent::Add(added_id, sent) = event else {
                panic!("{event:?}");
            };
            assert!(crate::mob_tracking::spawn_tracked(
                &mut friend,
                *added_id,
                sent
            ));
            added.push(*added_id);
        }
        assert_eq!(added, vec![id]);
        assert_eq!(friend.entity_type(id), Some("minecraft:breeze"));
        assert_eq!(friend.family_core(id).unwrap().1.yaw, 45.0);
        host.family_core_mut(id).unwrap().1.body.position = DVec3::new(4.0, 65.0, 1.5);
        trackers.tick();
        let later = trackers.update(&host, &originals, &[watcher]);
        for mob in later.moves_of(3).unwrap() {
            crate::mob_tracking::show_on(&mut friend, mob);
        }
        assert_eq!(
            friend.family_core(id).unwrap().1.body.position,
            DVec3::new(4.0, 65.0, 1.5)
        );
    }

    #[test]
    fn a_breeze_whirls_now_and_then_for_every_viewer() {
        use crate::ambient_particles::AmbientParticles;
        use crate::poof_particles::PoofParticles;
        use crate::portal_particles::PortalParticles;
        use crate::spell_particles::SpellParticles;
        use minecraftoss_player::rng::LegacyRandom;
        let mut world = EntityWorld::default();
        let tag = parse("{id:\"minecraft:breeze\",Pos:[0.5d,64.0d,0.5d],NoAI:1b}");
        crate::server_mobs::spawn_saved(&mut world, &tag).unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let mut ambient = AmbientParticles::new(1);
        let mut spell = SpellParticles::new(1);
        let mut portal = PortalParticles::default();
        let mut poof = PoofParticles::default();
        let mut sounds = Vec::new();
        let mut random = LegacyRandom::new(1);
        let mut seen = HashMap::new();
        let mut plays = Vec::new();
        for tick in 1..=600 {
            let mut fx = ClientFx {
                ambient: &mut ambient,
                spell: &mut spell,
                portal: &mut portal,
                poof: &mut poof,
                sounds: &mut sounds,
                random: &mut random,
                seen: &mut seen,
            };
            crate::families::client_effects(&world, &poses, &mut fx);
            for (event, at, volume, pitch) in sounds.drain(..) {
                assert_eq!(event, WHIRL_SOUND);
                assert_eq!(at, DVec3::new(0.5, 64.0, 0.5));
                assert!((0.8..=1.0).contains(&volume) && (0.7..=1.1).contains(&pitch));
                plays.push(tick);
            }
        }
        assert!(plays.len() > 5, "{plays:?}");
        assert!(plays[0] >= 2 && plays[0] <= 81);
        assert!(
            plays
                .windows(2)
                .all(|pair| (2..=81).contains(&(pair[1] - pair[0]))),
            "{plays:?}"
        );
    }
}
