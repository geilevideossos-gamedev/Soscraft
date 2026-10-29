use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
use minecraftoss_entities::look::BodyRotation;
use minecraftoss_entities::villager_brain::Slot;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::deep_dark::wardens::Warden;
use minecraftoss_entities::world::families::deep_dark::wardens::{self, Anger, Memories, Pose};
use std::collections::BTreeMap;

pub const TRACKING_RANGE: i32 = 16;
pub const SHADOW_RADIUS: f32 = 0.9;
pub const ANGER: &str = "anger";
pub const SUSPECTS: &str = "suspects";
pub const BRAIN: &str = "Brain";
pub const MEMORIES: &str = "memories";
const POSE: usize = 0;
const POSE_AGE: usize = 1;
const CLIENT_ANGER: usize = 2;

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
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

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
        let mut show = FamilyShow::default();
        show.values[POSE] = mob.state.pose.id();
        show.values[POSE_AGE] = mob.state.pose_age;
        show.values[CLIENT_ANGER] = mob.state.client_anger;
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        wardens::set_pose(mob, Pose::of_id(show.values[POSE]));
        mob.state.pose_age = show.values[POSE_AGE];
        mob.state.client_anger = show.values[CLIENT_ANGER];
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_tracking::{look, shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_entities::monster_ai::Target;
    use minecraftoss_entities::world::EntityWorld;
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
        mob.state.pose_age = 30;
        mob.state.client_anger = 85;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(show.values[..3], [Pose::Emerging.id(), 30, 85]);
        Warden::show_on(friend.warden_mut(id).unwrap(), &show);
        let mirror = warden(&friend, id);
        assert_eq!(
            (mirror.state.pose, mirror.state.pose_age),
            (Pose::Emerging, 30)
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
    }
}
