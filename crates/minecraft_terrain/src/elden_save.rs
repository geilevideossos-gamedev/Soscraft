use crate::families::Common;
use elden_boss::BossDef;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::bosses::elden_bosses::{self, EldenBoss};
use std::collections::BTreeMap;

pub const CARRIER: &str = "minecraft:marker";
pub const PEACEFUL: &str = "You can not summon a monster in peaceful mode";

fn carried(tag: &Tag) -> Option<&Tag> {
    tag.get("data")?.get("iw4l")
}

pub fn boss_of(tag: &Tag) -> Option<&'static BossDef> {
    elden_boss::def(carried(tag)?.get("boss")?.as_str()?)
}

pub fn claims(tag: &Tag) -> bool {
    tag.get("id").and_then(Tag::as_str) == Some(CARRIER) && boss_of(tag).is_some()
}

pub fn spawn(world: &mut EntityWorld, tag: &Tag, common: &Common) -> Option<u64> {
    if !claims(tag) {
        return None;
    }
    let def = boss_of(tag)?;
    let id = world.spawn_elden_boss(def, common.pos, common.yaw, common.pos, common.no_ai);
    let mob = world.elden_boss_mut(id)?;
    mob.core.random = common.random.clone();
    mob.core.body.fire_ticks = common.fire;
    read(mob, tag);
    Some(id)
}

pub fn read(mob: &mut FamilyMob<EldenBoss>, tag: &Tag) {
    let carried = carried(tag);
    let anchor = carried.and_then(|iw4l| iw4l.get("anchor")).and_then(point);
    mob.state.anchor = anchor.unwrap_or(mob.core.body.position);
    let rest = carried
        .and_then(|iw4l| iw4l.get("yaw"))
        .and_then(Tag::as_f64);
    mob.state.rest_yaw = rest.map_or(mob.core.yaw, |yaw| yaw as f32);
    mob.state.brain.reset(elden_bosses::tick_of(&mob.core));
    mob.state.shown = mob.state.brain.clip();
    mob.core.max_health = mob.state.brain.max_health();
    mob.core.health = mob.state.brain.health();
    mob.core.persistent = true;
}

pub fn summon(
    world: &mut EntityWorld,
    def: &'static BossDef,
    position: DVec3,
    y_rot: f32,
    nbt: Option<&Tag>,
    uuid: [i32; 4],
) -> (u64, Tag) {
    let given = |key: &str| nbt.and_then(|tag| tag.get(key));
    let no_ai = given("NoAI")
        .and_then(Tag::as_i64)
        .is_some_and(|flag| flag != 0);
    let id = world.spawn_elden_boss(def, position, y_rot, position, no_ai);
    world.set_uuid(id, minecraftoss_entities::gossip::uuid_from_ints(uuid));
    let mut map = BTreeMap::from([
        ("Pos".to_owned(), point_tag(position)),
        (
            "Rotation".to_owned(),
            Tag::List(vec![Tag::Float(y_rot), Tag::Float(0.0)]),
        ),
        ("UUID".to_owned(), Tag::IntArray(uuid.to_vec())),
    ]);
    if let Some(mob) = world.elden_boss_mut(id) {
        if let Some(health) = given("Health").and_then(Tag::as_f64) {
            mob.state.brain.set_health(health as f32);
            mob.core.health = mob.state.brain.health();
        }
        for (key, value) in fields(mob) {
            map.insert(key.to_owned(), value);
        }
    }
    (id, Tag::Compound(map))
}

pub fn point_tag(at: DVec3) -> Tag {
    Tag::List(vec![
        Tag::Double(at.x),
        Tag::Double(at.y),
        Tag::Double(at.z),
    ])
}

pub fn point(tag: &Tag) -> Option<DVec3> {
    match tag.as_list()? {
        [x, y, z] => Some(DVec3::new(x.as_f64()?, y.as_f64()?, z.as_f64()?)),
        _ => None,
    }
}

pub fn fields(mob: &FamilyMob<EldenBoss>) -> Vec<(&'static str, Tag)> {
    let anchor = point_tag(mob.state.anchor);
    let iw4l = BTreeMap::from([
        ("boss".to_owned(), Tag::String(mob.state.def.id.to_owned())),
        ("anchor".to_owned(), anchor),
        ("yaw".to_owned(), Tag::Float(mob.state.rest_yaw)),
    ]);
    let data = BTreeMap::from([("iw4l".to_owned(), Tag::Compound(iw4l))]);
    vec![
        ("id", Tag::String(CARRIER.to_owned())),
        ("data", Tag::Compound(data)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_mobs::{mob_tags_where, spawn_saved};
    use elden_boss::State;
    use elden_boss::margit::MARGIT;
    use minecraftoss_entities::tempt::PlayerCandidate;
    use minecraftoss_player::{Block, Pos, World};
    use std::collections::HashMap;

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).expect("the tag")
    }

    fn saved_tag(world: &EntityWorld, id: u64) -> Tag {
        let tags = mob_tags_where(world, |_, _| true, &HashMap::new(), false);
        let (_, tag) = tags
            .into_iter()
            .find(|(saved, _)| *saved == id)
            .expect("saved");
        tag
    }

    fn margit(world: &EntityWorld, id: u64) -> &FamilyMob<EldenBoss> {
        let bosses = world.elden_bosses();
        bosses.iter().find(|mob| mob.core.id == id).expect("Margit")
    }

    fn vanilla_ids(tag: &Tag) -> bool {
        match tag {
            Tag::Compound(map) => map.iter().all(|(key, value)| {
                let named =
                    key != "id" || value.as_str().is_none_or(|id| id.starts_with("minecraft:"));
                named && vanilla_ids(value)
            }),
            Tag::List(items) => items.iter().all(vanilla_ids),
            _ => true,
        }
    }

    #[test]
    fn margits_saved_tag_is_a_vanilla_marker() {
        let mut world = EntityWorld::default();
        let anchor = DVec3::new(4.5, 64.0, -3.5);
        let at = DVec3::new(10.5, 64.0, 2.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 45.0, anchor, false);
        let tag = saved_tag(&world, id);
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(CARRIER));
        assert!(claims(&tag));
        let iw4l = carried(&tag).expect("data.iw4l");
        assert_eq!(iw4l.get("boss").and_then(Tag::as_str), Some("iw4l:margit"));
        assert_eq!(iw4l.get("anchor").and_then(point), Some(anchor));
        assert_eq!(iw4l.get("yaw").and_then(Tag::as_f64), Some(45.0));
        assert!(vanilla_ids(&tag), "{tag:?}");
        assert_eq!(
            tag.get("PersistenceRequired").and_then(Tag::as_i64),
            Some(1)
        );
        let look = crate::mob_tracking::look(&tag);
        assert_eq!(
            look.get("data"),
            tag.get("data"),
            "the look tag keeps his anchor"
        );
        assert!(claims(&look));
    }

    #[test]
    fn a_hurt_margit_loads_at_full_health_and_walks_home() {
        let mut world = EntityWorld::default();
        let anchor = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, anchor, 0.0, anchor, false);
        let players = [player_at(1, DVec3::new(6.5, 64.0, 0.5))];
        for _ in 0..40 {
            world.tick_with_players(&mut Field, &players);
        }
        let away = DVec3::new(8.5, 64.0, 0.5);
        let mob = world.elden_boss_mut(id).unwrap();
        assert!(mob.state.brain.awake());
        mob.state.brain.set_health(300.0);
        mob.core.health = 300.0;
        mob.core.body.position = away;
        let tag = saved_tag(&world, id);
        let mut fresh = EntityWorld::default();
        let back = spawn_saved(&mut fresh, &tag).expect("Margit loads from his marker");
        assert_eq!(fresh.entity_type(back), Some("iw4l:margit"));
        let mob = margit(&fresh, back);
        assert_eq!(mob.core.body.position, away);
        assert_eq!(mob.state.anchor, anchor);
        assert_eq!((mob.core.health, mob.core.max_health), (800.0, 800.0));
        assert_eq!(mob.state.brain.health(), 800.0);
        assert_eq!(mob.state.brain.phase(), 1);
        assert_eq!(mob.state.brain.state(), State::Return);
        assert!(!mob.state.brain.awake());
        fresh.tick_with_players(&mut Field, &[]);
        let mob = margit(&fresh, back);
        assert_eq!(mob.state.brain.state(), State::Return);
        assert_eq!(mob.state.walk, Some(anchor), "he heads home");
        let again = saved_tag(&fresh, back);
        assert_eq!(carried(&again), carried(&tag), "the same carrier again");
    }

    #[test]
    fn a_summoned_margit_sleeps_with_his_anchor_there_and_a_given_health() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(3.5, 64.0, -2.5);
        let uuid = [1, 2, 3, 4];
        let (id, tag) = summon(&mut world, &MARGIT, at, 30.0, None, uuid);
        let mob = margit(&world, id);
        assert_eq!(mob.state.brain.state(), State::Asleep);
        assert_eq!((mob.state.anchor, mob.core.body.position), (at, at));
        assert_eq!((mob.core.yaw, mob.state.rest_yaw), (30.0, 30.0));
        assert_eq!(mob.core.health, 800.0);
        assert!(claims(&tag));
        assert_eq!(tag.get("Pos").and_then(point), Some(at));
        assert_eq!(tag.get("UUID").and_then(Tag::as_ints), Some(uuid.to_vec()));
        let ints = minecraftoss_entities::gossip::uuid_to_ints(world.uuid_of(id));
        assert_eq!(ints, uuid);
        let given = parse("{Health:401.0f,NoAI:1b}");
        let (half, _) = summon(&mut world, &MARGIT, at, 0.0, Some(&given), [5, 6, 7, 8]);
        let mob = margit(&world, half);
        assert_eq!((mob.core.health, mob.state.brain.health()), (401.0, 401.0));
        assert!(mob.core.no_ai);
    }

    #[test]
    fn a_plain_marker_stays_dormant() {
        let mut world = EntityWorld::default();
        for text in [
            "{id:\"minecraft:marker\",Pos:[0.5d,64.0d,0.5d]}",
            "{id:\"minecraft:marker\",Pos:[0.5d,64.0d,0.5d],data:{note:\"hi\"}}",
            "{id:\"minecraft:marker\",Pos:[0.5d,64.0d,0.5d],data:{iw4l:{boss:\"iw4l:morgott\"}}}",
            "{id:\"minecraft:armor_stand\",Pos:[0.5d,64.0d,0.5d],data:{iw4l:{boss:\"iw4l:margit\"}}}",
        ] {
            let tag = parse(text);
            assert!(!claims(&tag), "{text}");
            assert_eq!(spawn_saved(&mut world, &tag), None, "{text}");
        }
        assert_eq!(world.mob_ids().count(), 0);
    }

    #[test]
    fn a_saved_health_is_never_read() {
        let mut world = EntityWorld::default();
        let marker = parse(
            "{id:\"minecraft:marker\",Pos:[2.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],Health:5.0f,data:{iw4l:{boss:\"iw4l:margit\"}}}",
        );
        let id = spawn_saved(&mut world, &marker).expect("loads");
        let mob = margit(&world, id);
        assert_eq!(mob.core.health, 800.0);
        assert_eq!(
            mob.state.anchor,
            DVec3::new(2.5, 64.0, 2.5),
            "no anchor: where he stands"
        );
        assert_eq!((mob.core.yaw, mob.state.rest_yaw), (90.0, 90.0));
        let bare = parse(
            "{id:\"iw4l:margit\",Pos:[0.5d,64.0d,0.5d],Health:5.0f,data:{iw4l:{anchor:[1.5d,64.0d,1.5d]}}}",
        );
        let id = spawn_saved(&mut world, &bare).expect("loads by his own id");
        let mob = margit(&world, id);
        assert_eq!(mob.core.health, 800.0);
        assert_eq!(mob.state.anchor, DVec3::new(1.5, 64.0, 1.5));
        assert_eq!(mob.state.brain.state(), State::Return);
    }
}
