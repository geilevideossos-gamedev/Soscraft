use crate::block_particles::BlockDust;
use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::look::BodyRotation;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::trial::breezes::Breeze;
use minecraftoss_entities::world::families::trial::breezes::{MAX_HEAD_Y_ROT, Pose};

mod render;

pub const WHIRL_SOUND: &str = "minecraft:entity.breeze.whirl";
pub const WHIRL_MAX_TICKS: u32 = 80;
pub const GROUND_DUST: u32 = 1;
pub const SLIDE_DUST: u32 = 20;
pub const JUMP_TRAIL_DUST: u32 = 3;
pub const JUMP_TRAIL_TICKS: i32 = 5;
pub const JUMP_TRAIL_LIFT: f32 = 0.1;
const POSE: usize = 6;
const POSES: [Pose; 5] = [
    Pose::Standing,
    Pose::Shooting,
    Pose::Inhaling,
    Pose::Sliding,
    Pose::LongJumping,
];

fn pose_id(pose: Pose) -> i32 {
    POSES.iter().position(|known| *known == pose).unwrap_or(0) as i32
}

fn pose_of(id: i32) -> Pose {
    usize::try_from(id)
        .ok()
        .and_then(|id| POSES.get(id))
        .copied()
        .unwrap_or_default()
}

impl TerrainFamily for Breeze {
    const TRACKING_RANGE: i32 = 10;
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = render::SHADOW_RADIUS;
    const MAX_HEAD_Y_ROT: f32 = MAX_HEAD_Y_ROT;

    fn read(mob: &mut FamilyMob<Self>, _tag: &Tag) {
        mob.state.body_rotation = BodyRotation::new(mob.core.yaw);
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let now = mob.core.tick_count;
        let mut show = FamilyShow::default();
        for (index, start) in mob.state.animations.starts().into_iter().enumerate() {
            if let Some(start) = start {
                show.flags |= 1 << index;
                show.values[index] = now - start;
            }
        }
        show.values[POSE] = pose_id(mob.state.animations.pose);
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        let now = mob.core.tick_count;
        let starts = std::array::from_fn(|index| {
            (show.flags & (1 << index) != 0).then(|| now - show.values[index])
        });
        let pose = pose_of(show.values[POSE]);
        let animations = &mut mob.state.animations;
        animations.set_starts(starts);
        animations.pose = pose;
        mob.state.brain.pose = pose;
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        let shown = mobs.iter().filter(|mob| !mob.core.removed);
        let clips = render::Clips::shared();
        render::append_breezes(out, shown, poses, atlas, light, partial, &clips);
    }

    fn block_dust(mobs: &[FamilyMob<Self>], poses: &ClientMobs, out: &mut Vec<BlockDust>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let id = mob.core.id;
            let Some(shown) = poses.get(id) else {
                continue;
            };
            let feet = shown.position;
            let riding = poses.pose(id, 0.0).is_some_and(|pose| pose.riding);
            let animations = &mob.state.animations;
            match animations.pose {
                Pose::Standing | Pose::Shooting | Pose::Inhaling if !riding => {
                    out.push(BlockDust::of(feet, feet, GROUND_DUST));
                }
                Pose::Sliding if !riding => out.push(BlockDust::of(feet, feet, SLIDE_DUST)),
                Pose::LongJumping => {
                    let jumped = animations
                        .long_jump
                        .map(|start| mob.core.tick_count - start);
                    if jumped.is_some_and(|ticks| (0..JUMP_TRAIL_TICKS).contains(&ticks)) {
                        let motion = shown.position - shown.old_position;
                        let at = feet + motion + DVec3::Y * f64::from(JUMP_TRAIL_LIFT);
                        out.push(BlockDust::of(feet, at, JUMP_TRAIL_DUST));
                    }
                }
                _ => {}
            }
        }
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

    #[test]
    fn a_friend_sees_a_breeze_animate_from_the_same_ticks_in_its_pose() {
        let mut host = EntityWorld::default();
        let id = host
            .spawn_family_kind("minecraft:breeze", DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        let mob = host.family_mob_mut::<Breeze>(id).unwrap();
        mob.core.tick_count = 40;
        mob.state.animations.tick(Pose::Sliding, 35);
        mob.state.animations.tick(Pose::Standing, 38);
        mob.state.animations.tick(Pose::Shooting, 40);
        mob.state.brain.pose = Pose::Shooting;
        let sent = <Breeze as TerrainFamily>::show(mob);
        assert_eq!(sent.flags, 0b1011);
        assert_eq!(sent.values, [0, 1, 0, 2, 0, 0, 1, 0]);
        let mut friend = EntityWorld::default();
        let mirror = friend
            .spawn_family_kind("minecraft:breeze", DVec3::new(0.5, 64.0, 0.5))
            .unwrap();
        let shown = friend.family_mob_mut::<Breeze>(mirror).unwrap();
        shown.core.tick_count = 7;
        <Breeze as TerrainFamily>::show_on(shown, &sent);
        let host_mob = &host.family::<Breeze>()[0];
        let elapsed = |mob: &FamilyMob<Breeze>| {
            let now = mob.core.tick_count;
            mob.state
                .animations
                .starts()
                .map(|start| start.map(|start| now - start))
        };
        assert_eq!(elapsed(shown), elapsed(host_mob));
        assert_eq!(
            (shown.state.animations.pose, shown.state.brain.pose),
            (Pose::Shooting, Pose::Shooting)
        );
        for (pose, id) in POSES.iter().zip(0..) {
            assert_eq!((pose_id(*pose), pose_of(id)), (id, *pose));
        }
        assert_eq!(pose_of(-1), Pose::Standing);
        assert_eq!(pose_of(99), Pose::Standing);
    }

    #[test]
    fn a_breeze_raises_dust_where_it_stands_slides_and_leaps() {
        let mut world = EntityWorld::default();
        let feet = DVec3::new(2.5, 64.0, -1.5);
        let id = world.spawn_family_kind("minecraft:breeze", feet).unwrap();
        world.family_core_mut(id).unwrap().1.tick_count = 20;
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let mut dust = |pose: Pose, jumped: Option<i32>| {
            let mob = world.family_mob_mut::<Breeze>(id).unwrap();
            mob.state.animations.pose = pose;
            mob.state.animations.long_jump = jumped.map(|ticks| 20 - ticks);
            let mut out = Vec::new();
            crate::families::block_dust(&world, &poses, &mut out);
            out
        };
        let standing = dust(Pose::Standing, None);
        assert_eq!(standing, [BlockDust::of(feet, feet, GROUND_DUST)]);
        assert_eq!(
            (standing[0].inside, standing[0].below),
            ((2, 64, -2), (2, 63, -2))
        );
        assert_eq!(dust(Pose::Shooting, None), standing);
        assert_eq!(dust(Pose::Inhaling, None), standing);
        assert_eq!(
            dust(Pose::Sliding, None),
            [BlockDust::of(feet, feet, SLIDE_DUST)]
        );
        let trail = BlockDust::of(feet, feet + DVec3::Y * f64::from(0.1_f32), JUMP_TRAIL_DUST);
        for ticks in 0..JUMP_TRAIL_TICKS {
            assert_eq!(dust(Pose::LongJumping, Some(ticks)), [trail], "{ticks}");
        }
        assert!(dust(Pose::LongJumping, Some(JUMP_TRAIL_TICKS)).is_empty());
        assert!(dust(Pose::LongJumping, None).is_empty());
        let mut none = Vec::new();
        crate::families::block_dust(&world, &ClientMobs::default(), &mut none);
        assert!(none.is_empty(), "a breeze the viewer has not seen");
    }
}
