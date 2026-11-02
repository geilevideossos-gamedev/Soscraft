use super::warden_render;
use crate::block_particles::BlockDust;
use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
use minecraftoss_entities::look::BodyRotation;
use minecraftoss_entities::villager_brain::Slot;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::deep_dark::wardens::Warden;
use minecraftoss_entities::world::families::deep_dark::wardens::{self, Anger, Memories, Pose};
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

pub const TRACKING_RANGE: i32 = 16;
pub const HEARTBEAT_SOUND: &str = "minecraft:entity.warden.heartbeat";
pub const HEARTBEAT_VOLUME: f32 = 5.0;
pub const DIGGING_PARTICLES: u32 = 30;
pub const DIGGING_PARTICLES_MILLIS: f32 = 4500.0;
pub const DIGGING_PARTICLES_OFFSET: f32 = 0.7;
pub const ANGER: &str = "anger";
pub const SUSPECTS: &str = "suspects";
pub const BRAIN: &str = "Brain";
pub const MEMORIES: &str = "memories";
const CLIENT_ANGER: usize = 0;
const CLOCKS: usize = 1;
const POSE_SHIFT: u32 = 8;

type Flag = fn(&mut Memories) -> &mut Slot<()>;

const FLAGS: [(&str, Flag); 12] = [
    ("minecraft:dig_cooldown", |m| &mut m.dig_cooldown),
    ("minecraft:is_emerging", |m| &mut m.is_emerging),
    ("minecraft:is_sniffing", |m| &mut m.is_sniffing),
    ("minecraft:recent_projectile", |m| &mut m.recent_projectile),
    ("minecraft:roar_sound_cooldown", |m| {
        &mut m.roar_sound_cooldown
    }),
    ("minecraft:roar_sound_delay", |m| &mut m.roar_sound_delay),
    ("minecraft:sniff_cooldown", |m| &mut m.sniff_cooldown),
    ("minecraft:sonic_boom_cooldown", |m| {
        &mut m.sonic_boom_cooldown
    }),
    ("minecraft:sonic_boom_sound_cooldown", |m| {
        &mut m.sonic_boom_sound_cooldown
    }),
    ("minecraft:sonic_boom_sound_delay", |m| {
        &mut m.sonic_boom_sound_delay
    }),
    ("minecraft:touch_cooldown", |m| &mut m.touch_cooldown),
    ("minecraft:vibration_cooldown", |m| {
        &mut m.vibration_cooldown
    }),
];

fn suspects(tag: &Tag) -> Vec<(u128, i32)> {
    let Some(list) = tag
        .get(ANGER)
        .and_then(|anger| anger.get(SUSPECTS))
        .and_then(Tag::as_list)
    else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|suspect| {
            let ints = suspect.get("uuid").and_then(Tag::as_ints)?;
            let uuid = uuid_from_ints(<[i32; 4]>::try_from(ints).ok()?);
            let anger = suspect.get("anger").and_then(Tag::as_i64)?;
            Some((uuid, anger.clamp(0, i64::from(i32::MAX)) as i32))
        })
        .collect()
}

fn anger_tag(anger: &Anger) -> Tag {
    let mut map = BTreeMap::new();
    let saved = anger.saved();
    if !saved.is_empty() {
        let list = saved
            .into_iter()
            .map(|(uuid, anger)| {
                let suspect = [
                    (
                        "uuid".to_owned(),
                        Tag::IntArray(uuid_to_ints(uuid).to_vec()),
                    ),
                    ("anger".to_owned(), Tag::Int(anger)),
                ];
                Tag::Compound(suspect.into())
            })
            .collect();
        map.insert(SUSPECTS.to_owned(), Tag::List(list));
    }
    Tag::Compound(map)
}

fn read_memories(tag: &Tag, memories: &mut Memories) {
    let Some(saved) = tag.get(BRAIN).and_then(|brain| brain.get(MEMORIES)) else {
        return;
    };
    for (key, slot) in FLAGS {
        let Some(memory) = saved.get(key) else {
            continue;
        };
        match memory.get("ttl").and_then(Tag::as_i64) {
            Some(ttl) => slot(memories).set_for((), ttl),
            None => slot(memories).set(()),
        }
    }
}

fn throws_up_ground(mob: &FamilyMob<Warden>) -> bool {
    let start = match mob.state.pose {
        Pose::Emerging => mob.state.animations.emerge,
        Pose::Digging => mob.state.animations.dig,
        _ => None,
    };
    start.is_some_and(|start| {
        ((mob.core.tick_count - start) as f32 * 50.0) < DIGGING_PARTICLES_MILLIS
    })
}

fn brain_tag(memories: &Memories) -> Tag {
    let mut memories = memories.clone();
    let mut saved = BTreeMap::new();
    for (key, slot) in FLAGS {
        let slot = slot(&mut memories);
        if !slot.present() {
            continue;
        }
        let mut memory = BTreeMap::new();
        memory.insert("value".to_owned(), Tag::Compound(BTreeMap::new()));
        if let Some(ttl) = slot.ttl() {
            memory.insert("ttl".to_owned(), Tag::Long(ttl));
        }
        saved.insert(key.to_owned(), Tag::Compound(memory));
    }
    Tag::Compound([(MEMORIES.to_owned(), Tag::Compound(saved))].into())
}

impl TerrainFamily for Warden {
    const TRACKING_RANGE: i32 = TRACKING_RANGE;
    const VOLATILE: &'static [&'static str] = &[ANGER, crate::vibrations::LISTENER];
    const TEXTURES: &'static [&'static str] = &warden_render::TEXTURES;
    const SHADOW_RADIUS: f32 = warden_render::SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        mob.state.body_rotation = BodyRotation::new(mob.core.yaw);
        mob.state.anger = Anger::loaded(suspects(tag), 0);
        read_memories(tag, &mut mob.state.brain.memories);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        vec![
            (ANGER, anger_tag(&mob.state.anger)),
            (BRAIN, brain_tag(&mob.state.brain.memories)),
        ]
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let now = mob.core.tick_count;
        let mut show = FamilyShow {
            flags: (mob.state.pose.id() as u32) << POSE_SHIFT,
            ..FamilyShow::default()
        };
        for (index, start) in mob.state.animations.starts().into_iter().enumerate() {
            if let Some(start) = start {
                show.flags |= 1 << index;
                show.values[CLOCKS + index] = now - start;
            }
        }
        show.values[CLIENT_ANGER] = mob.state.client_anger;
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        wardens::set_pose(mob, Pose::of_id((show.flags >> POSE_SHIFT) as i32));
        let now = mob.core.tick_count;
        let starts = std::array::from_fn(|index| {
            (show.flags & (1 << index) != 0).then(|| now - show.values[CLOCKS + index])
        });
        mob.state.animations.set_starts(starts);
        mob.state.client_anger = show.values[CLIENT_ANGER];
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
        let clips = warden_render::Clips::shared();
        warden_render::append_wardens(out, shown, poses, atlas, light, partial, &clips);
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            if wardens::heart_beats(shown.tick_count, mob.state.client_anger) {
                let pitch = (fx.random.next_float() - fx.random.next_float()) * 0.2 + 1.0;
                let sound = HEARTBEAT_SOUND.to_owned();
                fx.sounds
                    .push((sound, shown.position, HEARTBEAT_VOLUME, pitch));
            }
        }
    }

    fn block_dust(mobs: &[FamilyMob<Self>], poses: &ClientMobs, out: &mut Vec<BlockDust>) {
        for mob in mobs
            .iter()
            .filter(|mob| !mob.core.removed && throws_up_ground(mob))
        {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            let feet = shown.position;
            let seed = mob.core.id.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ shown.tick_count as u64;
            let mut random = LegacyRandom::new(seed);
            let spread = DIGGING_PARTICLES_OFFSET;
            for _ in 0..DIGGING_PARTICLES {
                let x = random.next_float() * (spread - -spread) + -spread;
                let z = random.next_float() * (spread - -spread) + -spread;
                let at = feet + DVec3::new(f64::from(x), 0.0, f64::from(z));
                out.push(BlockDust::of(feet, at, 1));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_tracking::{look, shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_entities::monster_ai::Target;
    use minecraftoss_entities::world::EntityWorld;
    use minecraftoss_entities::world::families::deep_dark::wardens::Animations;
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn warden(world: &EntityWorld, id: u64) -> &FamilyMob<Warden> {
        world
            .wardens()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn saved_tag(world: &EntityWorld, id: u64, tag: Tag) -> Tag {
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let mut saved = mob_tags(world, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        saved.remove(0).1
    }

    const SAVED: &str = "{id:\"minecraft:warden\",Pos:[1.5d,64.0d,2.5d],Rotation:[45.0f,0.0f],Health:420.0f,UUID:[I;9,9,9,9],anger:{suspects:[{uuid:[I;1,2,3,4],anger:90},{uuid:[I;5,6,7,8],anger:12}]},Brain:{memories:{\"minecraft:dig_cooldown\":{value:{},ttl:600L},\"minecraft:is_sniffing\":{value:{}},\"minecraft:recent_projectile\":{value:{},ttl:30L}}},listener:{event:{game_event:\"minecraft:step\",distance:6.5f,pos:[7.5d,64.0d,2.5d]},selector:{tick:-1L},event_delay:4}}";

    #[test]
    fn a_warden_saves_and_loads_its_anger_and_listener() {
        let tag = parse(SAVED);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a warden loads");
        assert_eq!(world.entity_type(id), Some(wardens::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(44));
        let mob = warden(&world, id);
        assert_eq!((mob.core.health, mob.core.yaw), (420.0, 45.0));
        assert_eq!(mob.state.body_rotation.body_yaw, 45.0);
        let one = uuid_from_ints([1, 2, 3, 4]);
        let five = uuid_from_ints([5, 6, 7, 8]);
        assert_eq!(mob.state.anger.by_uuid, vec![(one, 90), (five, 12)]);
        let memories = mob.state.memories();
        assert_eq!(memories.dig_cooldown.ttl(), Some(600));
        assert!(memories.is_sniffing.present() && memories.is_sniffing.ttl().is_none());
        assert_eq!(memories.recent_projectile.ttl(), Some(30));
        assert!(!memories.vibration_cooldown.present());
        let saved = saved_tag(&world, id, tag.clone());
        assert_eq!(
            saved.get(ANGER),
            tag.get(ANGER),
            "the anger goes back as it came"
        );
        assert_eq!(saved.get(BRAIN), tag.get(BRAIN));
        assert_eq!(
            saved.get(crate::vibrations::LISTENER),
            tag.get(crate::vibrations::LISTENER),
            "the listener rides in the tag the server keeps"
        );
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        world.set_uuid(77, one);
        let mob = world.warden_mut(id).unwrap();
        mob.state.anger.increase(Target::Mob(77), one, 10);
        let saved = saved_tag(&world, id, tag.clone());
        let anger = saved
            .get(ANGER)
            .and_then(|a| a.get(SUSPECTS))
            .and_then(Tag::as_list)
            .unwrap();
        assert_eq!(anger.len(), 2);
        assert_eq!(
            anger[0].get("anger").and_then(Tag::as_i64),
            Some(100),
            "a known suspect comes first"
        );
        assert_eq!(warden(&again, back).state.anger.by_uuid.len(), 2);
        let fresh = parse("{id:\"minecraft:warden\",Pos:[1.5d,64.0d,2.5d]}");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &fresh).unwrap();
        let saved = saved_tag(&world, id, fresh);
        assert_eq!(saved.get(ANGER), Some(&Tag::Compound(BTreeMap::new())));
        assert_eq!(saved.get(BRAIN), Some(&parse("{memories:{}}")));
    }

    #[test]
    fn a_warden_keeps_its_roar_and_boom_cooldowns_in_its_brain() {
        let tag = parse(
            "{id:\"minecraft:warden\",Pos:[1.5d,64.0d,2.5d],Brain:{memories:{\"minecraft:roar_sound_cooldown\":{value:{},ttl:40L},\"minecraft:roar_sound_delay\":{value:{},ttl:3L},\"minecraft:sonic_boom_cooldown\":{value:{},ttl:150L},\"minecraft:sonic_boom_sound_cooldown\":{value:{},ttl:20L},\"minecraft:sonic_boom_sound_delay\":{value:{},ttl:7L}}}}",
        );
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).unwrap();
        let memories = warden(&world, id).state.memories();
        assert_eq!(memories.roar_sound_cooldown.ttl(), Some(40));
        assert_eq!(memories.roar_sound_delay.ttl(), Some(3));
        assert_eq!(memories.sonic_boom_cooldown.ttl(), Some(150));
        assert_eq!(memories.sonic_boom_sound_cooldown.ttl(), Some(20));
        assert_eq!(memories.sonic_boom_sound_delay.ttl(), Some(7));
        let saved = saved_tag(&world, id, tag.clone());
        assert_eq!(saved.get(BRAIN), tag.get(BRAIN));
    }

    #[test]
    fn a_friend_sees_a_warden_emerge() {
        let mut host = EntityWorld::default();
        let tag = parse(SAVED);
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, tag);
        let seen = look(&saved);
        for key in [ANGER, crate::vibrations::LISTENER, BRAIN] {
            assert_eq!(seen.get(key), None, "{key} changes while it lives");
        }
        assert_eq!(crate::families::tracking_range(wardens::KIND), Some(16));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        assert_eq!(shows(&host)[&id].family, Some(FamilyShow::default()));
        let mob = host.warden_mut(id).unwrap();
        wardens::emerge(mob);
        mob.core.tick_count = 30;
        mob.state.client_anger = 85;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(show.flags, 1 | (Pose::Emerging.id() as u32) << POSE_SHIFT);
        assert_eq!(show.values[..2], [85, 30]);
        Warden::show_on(friend.warden_mut(id).unwrap(), &show);
        let mirror = warden(&friend, id);
        let emerged = mirror.state.animations.emerge;
        assert_eq!(
            (
                mirror.state.pose,
                emerged.map(|start| mirror.core.tick_count - start)
            ),
            (Pose::Emerging, Some(30))
        );
        assert_eq!(
            (mirror.core.body.height, mirror.state.client_anger),
            (1.0, 85)
        );
        Warden::show_on(friend.warden_mut(id).unwrap(), &FamilyShow::default());
        let mirror = warden(&friend, id);
        assert_eq!(
            (mirror.state.pose, mirror.core.body.height),
            (Pose::Standing, 2.9)
        );
        assert_eq!(mirror.state.animations, Animations::default());
    }

    fn clips() -> warden_render::Clips {
        use crate::keyframes::{Channel, Definition, Interpolation, Keyframe, Target, degree_vec};
        let mut clips = warden_render::Clips::default();
        for (index, bone) in [(0, "right_arm"), (1, "body"), (4, "head"), (5, "left_arm")] {
            let mut clip = Definition::new(4.0, false);
            let turn = degree_vec(-30.0 - index as f32, 0.0, 10.0);
            let keys = vec![
                Keyframe::new(0.0, degree_vec(0.0, 0.0, 0.0), Interpolation::Linear),
                Keyframe::new(4.0, turn, Interpolation::Linear),
            ];
            clip.add(bone, Channel::new(Target::Rotation, keys));
            clips.clips[index] = Some(std::sync::Arc::new(clip));
        }
        clips
    }

    fn mesh_of(world: &EntityWorld, partial: f32) -> Vec<([f32; 3], [f32; 4])> {
        use crate::mesh::ChunkMesh;
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(world));
        let temp = crate::test_dir::tempdir().unwrap();
        std::fs::write(
            temp.path().join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let packs = crate::pack::PackStack::open(vec![temp.path().into()]).unwrap();
        let atlas = crate::mesh::make_atlas(&packs, Vec::new()).unwrap();
        let light =
            SkyLight::from_levels((-8, 56, -8), (17, 17, 17), vec![15; 4913], vec![0; 4913]);
        let (mut models, mut culled, mut translucent) = (
            ChunkMesh::default(),
            ChunkMesh::default(),
            ChunkMesh::default(),
        );
        let mut out = FamilyDraw {
            models: &mut models,
            culled: &mut culled,
            translucent: &mut translucent,
            held: &mut Vec::new(),
        };
        let wardens = world.family::<Warden>();
        warden_render::append_wardens(&mut out, wardens, &poses, &atlas, &light, partial, &clips());
        models
            .vertices
            .iter()
            .chain(&translucent.vertices)
            .map(|v| (v.position, v.color))
            .collect()
    }

    fn effects(
        world: &EntityWorld,
        poses: &ClientMobs,
        random: &mut LegacyRandom,
    ) -> Vec<(String, DVec3, f32, f32)> {
        use crate::ambient_particles::AmbientParticles;
        use crate::families::ClientFx;
        use crate::poof_particles::PoofParticles;
        use crate::portal_particles::PortalParticles;
        use crate::spell_particles::SpellParticles;
        let mut ambient = AmbientParticles::new(1);
        let mut spell = SpellParticles::new(1);
        let mut portal = PortalParticles::default();
        let mut poof = PoofParticles::default();
        let mut sounds = Vec::new();
        let mut seen = HashMap::new();
        let mut fx = ClientFx {
            ambient: &mut ambient,
            spell: &mut spell,
            portal: &mut portal,
            poof: &mut poof,
            sounds: &mut sounds,
            random,
            seen: &mut seen,
        };
        crate::families::client_effects(world, poses, &mut fx);
        sounds
    }

    #[test]
    fn its_heart_beats_with_the_shown_anger_for_every_viewer() {
        let tag = parse("{id:\"minecraft:warden\",Pos:[0.5d,64.0d,0.5d],NoAI:1b}");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let mut random = LegacyRandom::new(5);
        let mut beats = Vec::new();
        for tick in 1..=200 {
            poses.tick();
            if tick == 121 {
                world.warden_mut(id).unwrap().state.client_anger = 80;
            }
            for (event, at, volume, pitch) in effects(&world, &poses, &mut random) {
                assert_eq!(event, HEARTBEAT_SOUND);
                assert_eq!((at, volume), (DVec3::new(0.5, 64.0, 0.5), 5.0));
                assert!((0.8..=1.2).contains(&pitch), "{pitch}");
                beats.push(tick);
            }
        }
        assert_eq!(beats, [40, 80, 120, 130, 140, 150, 160, 170, 180, 190, 200]);
        world.warden_mut(id).unwrap().core.removed = true;
        for _ in 0..40 {
            poses.tick();
            assert!(effects(&world, &poses, &mut random).is_empty());
        }
        assert!(effects(&world, &ClientMobs::default(), &mut random).is_empty());
    }

    #[test]
    fn digging_and_emerging_throw_up_thirty_bits_of_ground_a_tick_for_four_and_a_half_seconds() {
        let feet = DVec3::new(2.5, 64.0, -1.5);
        let mut world = EntityWorld::default();
        let id = world.spawn_family_kind(wardens::KIND, feet).unwrap();
        let mut poses = ClientMobs::default();
        poses.spawn_missing(&crate::client_mobs::server_mobs(&world));
        let dust = |world: &EntityWorld, poses: &ClientMobs| {
            let mut out = Vec::new();
            crate::families::block_dust(world, poses, &mut out);
            out
        };
        assert!(
            dust(&world, &poses).is_empty(),
            "standing, it throws nothing"
        );
        for pose in [Pose::Emerging, Pose::Digging] {
            let mob = world.warden_mut(id).unwrap();
            wardens::set_pose(mob, Pose::Standing);
            mob.core.tick_count = 1000;
            wardens::set_pose(mob, pose);
            for (since, thrown) in [(0, true), (1, true), (89, true), (90, false), (300, false)] {
                world.warden_mut(id).unwrap().core.tick_count = 1000 + since;
                let out = dust(&world, &poses);
                if !thrown {
                    assert!(out.is_empty(), "{pose:?} {since}");
                    continue;
                }
                assert_eq!(out.len(), 30, "{pose:?} {since}");
                for bit in &out {
                    assert_eq!(
                        (bit.count, bit.inside, bit.below),
                        (1, (2, 64, -2), (2, 63, -2))
                    );
                    assert_eq!(bit.at.y, 64.0);
                    assert!((bit.at.x - 2.5).abs() <= 0.7 && (bit.at.z + 1.5).abs() <= 0.7);
                }
                assert!(
                    out.windows(2).all(|pair| pair[0].at != pair[1].at),
                    "each its own spot"
                );
            }
        }
        world.warden_mut(id).unwrap().core.tick_count = 1010;
        let before = dust(&world, &poses);
        poses.tick();
        assert_ne!(dust(&world, &poses), before, "new spots every tick");
        assert!(
            dust(&world, &ClientMobs::default()).is_empty(),
            "a warden the viewer has not seen"
        );
        let mob = world.warden_mut(id).unwrap();
        mob.state.animations.dig = None;
        assert!(dust(&world, &poses).is_empty(), "no dig clock, no ground");
    }

    #[test]
    fn a_friend_sees_a_warden_roar_and_boom_at_the_same_ticks() {
        let tag = parse("{id:\"minecraft:warden\",Pos:[0.5d,64.0d,0.5d],Rotation:[30.0f,0.0f]}");
        let mut host = EntityWorld::default();
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, tag);
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        friend.warden_mut(id).unwrap().core.tick_count = 7;
        let steps: [(i32, fn(&mut FamilyMob<Warden>, i32)); 6] = [
            (100, |mob, _| wardens::set_pose(mob, Pose::Sniffing)),
            (110, |mob, _| wardens::set_pose(mob, Pose::Roaring)),
            (130, |mob, now| mob.state.animations.tendril = Some(now)),
            (194, |mob, _| wardens::set_pose(mob, Pose::Standing)),
            (200, |mob, now| mob.state.animations.swing(now)),
            (203, |mob, now| mob.state.animations.sonic_boom = Some(now)),
        ];
        for (now, step) in steps {
            let mob = host.warden_mut(id).unwrap();
            mob.core.tick_count = now;
            step(mob, now);
            mob.state.client_anger = now / 2;
            for later in [now, now + 3] {
                host.warden_mut(id).unwrap().core.tick_count = later;
                let show = shows(&host)[&id].family.unwrap();
                Warden::show_on(friend.warden_mut(id).unwrap(), &show);
                let (seen, sent) = (warden(&friend, id), warden(&host, id));
                let since = |mob: &FamilyMob<Warden>| {
                    let at = mob.core.tick_count;
                    mob.state
                        .animations
                        .starts()
                        .map(|start| start.map(|start| at - start))
                };
                assert_eq!(since(seen), since(sent), "{later}");
                assert_eq!(
                    (seen.state.pose, seen.state.client_anger),
                    (sent.state.pose, sent.state.client_anger)
                );
                for partial in [0.0, 0.6] {
                    assert_eq!(
                        mesh_of(&friend, partial),
                        mesh_of(&host, partial),
                        "{later}"
                    );
                }
            }
        }
        let sent = warden(&host, id).state.animations;
        assert_eq!(
            (
                sent.sniff,
                sent.roar,
                sent.attack,
                sent.sonic_boom,
                sent.tendril
            ),
            (Some(100), None, Some(200), Some(203), Some(130)),
            "the swing ended the roar"
        );
        let host_mesh = mesh_of(&host, 0.0);
        host.warden_mut(id).unwrap().state.animations.attack = None;
        assert_ne!(mesh_of(&host, 0.0), host_mesh, "the swing's clip moves it");
    }
}
