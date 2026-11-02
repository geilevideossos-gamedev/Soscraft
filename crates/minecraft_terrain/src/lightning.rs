use crate::server_mobs::{MobTables, MobWorld};
use crate::terrain::BlockStates;
use crate::thrown::PlayerBody;
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::tempt::PlayerCandidate;
use minecraftoss_entities::world::{Bolt, EntityWorld, ThunderOutcome};
use minecraftoss_world::level::Level;
use minecraftoss_world::level::entity::EntityKind;
use minecraftoss_world::level::lightning::LightningHit;
use minecraftoss_world::natural_spawner::SpawnLevel;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

pub const BOLT: &str = "minecraft:lightning_bolt";
pub const DAMAGE: f32 = minecraftoss_world::level::lightning::DAMAGE;
const ZOMBIFIED_PIGLIN: &str = "minecraft:zombified_piglin";
const WITCH: &str = "minecraft:witch";
const CARRIED: [&str; 7] = [
    "Pos",
    "Motion",
    "Rotation",
    "OnGround",
    "fall_distance",
    "active_effects",
    "AbsorptionAmount",
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlayerStrike {
    pub player: u64,
    pub at: [f64; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BoltView {
    pub id: i32,
    pub position: DVec3,
    pub seed: i64,
    pub life: i32,
}

pub fn views(level: &Level<'_>) -> Vec<BoltView> {
    level
        .entities
        .iter()
        .filter(|e| !e.removed)
        .filter_map(|e| match &e.kind {
            EntityKind::LightningBolt(data) => Some(BoltView {
                id: e.id,
                position: DVec3::from_array(e.pos),
                seed: data.seed,
                life: data.life,
            }),
            _ => None,
        })
        .collect()
}

pub fn summon(level: &mut Level<'static>, position: [f64; 3]) -> Result<(), String> {
    level.add_lightning(position, false);
    Ok(())
}

pub fn summon_kinds() -> Vec<&'static str> {
    let mut kinds = crate::server_mobs::simulated_kinds();
    kinds.push(BOLT);
    kinds.sort_unstable();
    kinds.dedup();
    kinds
}

pub fn strike(
    level: &mut Level<'static>,
    mobs: &mut EntityWorld,
    tags: &mut HashMap<u64, Tag>,
    states: &BlockStates,
    tables: &MobTables,
    players: &[PlayerCandidate],
    struck: &mut Vec<PlayerStrike>,
) -> Vec<Tag> {
    let mut turned = Vec::new();
    for hit in level.take_lightning_hits() {
        mobs.set_difficulty(level.difficulty);
        let bolt = Bolt {
            uuid: hit.uuid,
            at: DVec3::from_array(hit.at),
        };
        let world = MobWorld {
            level: RefCell::new(&mut *level),
            states,
            tables,
        };
        let outcomes = mobs.thunder_hit(&bolt, &world);
        for outcome in outcomes {
            turned.extend(turn(level, mobs, tags, outcome, &hit));
        }
        struck.extend(players_in(&hit, players));
    }
    turned
}

fn turn(
    level: &mut Level<'static>,
    mobs: &mut EntityWorld,
    tags: &mut HashMap<u64, Tag>,
    outcome: ThunderOutcome,
    hit: &LightningHit,
) -> Option<Tag> {
    let ThunderOutcome::Convert { id, into, held } = outcome;
    let only = |mob: u64, _: DVec3| mob == id;
    let (_, saved) = crate::server_mobs::mob_tags_where(mobs, only, tags, false)
        .into_iter()
        .next()?;
    let mut tag = turned_tag(&saved, into, held);
    if into == WITCH {
        let at = |tag: &Tag| {
            let pos = tag.get("Pos").and_then(Tag::as_list).unwrap_or_default();
            let axis = |i: usize| pos.get(i).and_then(Tag::as_f64).unwrap_or(0.0);
            format!("x={:.2}, y={:.2}, z={:.2}", axis(0), axis(1), axis(2))
        };
        let [x, y, z] = hit.at;
        eprintln!(
            "Villager {} was struck by lightning {}.",
            format_args!("Villager['Villager'/{id}, {}]", at(&saved)),
            format_args!(
                "LightningBolt['Lightning Bolt'/{}, x={x:.2}, y={y:.2}, z={z:.2}]",
                hit.bolt
            ),
        );
        tag = level.finalize_conversion(tag);
    } else if let Tag::Compound(map) = &mut tag {
        let uuid = SpawnLevel::next_uuid(level);
        map.insert("UUID".to_owned(), Tag::IntArray(uuid.to_vec()));
    }
    mobs.remove_mobs(&[id]);
    tags.remove(&id);
    Some(tag)
}

pub fn turned_tag(saved: &Tag, into: &str, held: Option<&str>) -> Tag {
    let mut turned = crate::server_mobs::converted_zombie_tag(saved);
    let (Tag::Compound(map), Tag::Compound(from)) = (&mut turned, saved) else {
        return turned;
    };
    map.remove("equipment");
    map.remove("drop_chances");
    for key in CARRIED {
        if let Some(value) = from.get(key) {
            map.insert(key.to_owned(), value.clone());
        }
    }
    map.insert("id".to_owned(), Tag::String(into.to_owned()));
    map.insert("PersistenceRequired".to_owned(), Tag::Byte(1));
    if into == WITCH {
        map.remove("CanPickUpLoot");
    } else {
        map.entry("CanPickUpLoot".to_owned())
            .or_insert(Tag::Byte(0));
    }
    if into == ZOMBIFIED_PIGLIN {
        let age = from.get("Age").and_then(Tag::as_i64).unwrap_or(0);
        map.insert("IsBaby".to_owned(), Tag::Byte(i8::from(age < 0)));
    }
    if let Some(item) = held {
        let stack = BTreeMap::from([
            ("id".to_owned(), Tag::String(item.to_owned())),
            ("count".to_owned(), Tag::Int(1)),
        ]);
        let slots = BTreeMap::from([("mainhand".to_owned(), Tag::Compound(stack))]);
        map.insert("equipment".to_owned(), Tag::Compound(slots));
    }
    turned
}

pub fn players_in(hit: &LightningHit, players: &[PlayerCandidate]) -> Vec<PlayerStrike> {
    players
        .iter()
        .map(PlayerBody::of)
        .filter(|body| {
            let (min, max) = body.bounds();
            body.targetable() && hit.touches(min.to_array(), max.to_array())
        })
        .map(|body| PlayerStrike {
            player: body.id,
            at: hit.at,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player(id: u64, feet: [f64; 3]) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position: DVec3::from_array(feet),
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

    #[test]
    fn summon_accepts_lightning_bolt() {
        let kinds = summon_kinds();
        assert!(kinds.contains(&BOLT));
        assert!(kinds.contains(&"minecraft:cow"));
        assert!(
            kinds.windows(2).all(|pair| pair[0] < pair[1]),
            "sorted, each once"
        );
    }

    #[test]
    fn a_bolt_strikes_the_living_players_in_its_box() {
        let hit = LightningHit {
            bolt: 1,
            uuid: [1, 2, 3, 4],
            at: [0.5, 64.0, 0.5],
        };
        let spectator = PlayerCandidate {
            spectator: true,
            ..player(3, [0.5, 64.0, 0.5])
        };
        let dead = PlayerCandidate {
            alive: false,
            ..player(4, [0.5, 64.0, 0.5])
        };
        let players = [
            player(1, [2.5, 70.0, -2.5]),
            player(2, [4.5, 64.0, 0.5]),
            spectator,
            dead,
            player(5, [0.5, 54.5, 0.5]),
        ];
        assert_eq!(
            players_in(&hit, &players),
            vec![PlayerStrike {
                player: 1,
                at: hit.at
            }]
        );
    }

    struct Air;

    impl minecraftoss_player::World for Air {
        fn block(&self, _: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            None
        }

        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    fn saved(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).expect("the tag")
    }

    fn flag(tag: &Tag, key: &str) -> Option<i64> {
        tag.get(key).and_then(Tag::as_i64)
    }

    #[test]
    fn a_turned_pig_keeps_its_place_name_and_effects_but_not_its_saddle() {
        let pig = saved(
            "{id:\"minecraft:pig\",UUID:[I;1,2,3,4],Pos:[1.5d,64.0d,2.5d],Motion:[0.0d,-0.08d,0.0d],Rotation:[90f,0f],OnGround:1b,fall_distance:0.0d,Health:7.0f,Fire:-1s,Age:-24000,CustomName:\"Babe\",AbsorptionAmount:2.0f,CanPickUpLoot:0b,LeftHanded:1b,PersistenceRequired:0b,active_effects:[{id:\"minecraft:speed\",duration:200,amplifier:1b}],equipment:{saddle:{id:\"minecraft:saddle\",count:1}},drop_chances:{saddle:2.0f},attributes:[{id:\"minecraft:movement_speed\",base:0.25d}]}",
        );
        let turned = turned_tag(&pig, ZOMBIFIED_PIGLIN, Some("minecraft:golden_sword"));
        assert_eq!(
            turned.get("id").and_then(Tag::as_str),
            Some(ZOMBIFIED_PIGLIN)
        );
        for key in [
            "Pos",
            "Motion",
            "Rotation",
            "OnGround",
            "fall_distance",
            "CustomName",
            "AbsorptionAmount",
            "LeftHanded",
            "active_effects",
        ] {
            assert_eq!(turned.get(key), pig.get(key), "{key}");
        }
        for key in [
            "UUID",
            "Health",
            "Fire",
            "Age",
            "drop_chances",
            "attributes",
        ] {
            assert_eq!(turned.get(key), None, "{key}");
        }
        assert_eq!(flag(&turned, "IsBaby"), Some(1), "a baby pig gives a baby");
        assert_eq!(flag(&turned, "PersistenceRequired"), Some(1));
        assert_eq!(flag(&turned, "CanPickUpLoot"), Some(0));
        let equipment = turned.get("equipment").expect("its sword");
        assert_eq!(equipment.get("saddle"), None, "the saddle is lost");
        let held = equipment.get("mainhand").and_then(|item| item.get("id"));
        assert_eq!(held.and_then(Tag::as_str), Some("minecraft:golden_sword"));
        let adult = turned_tag(
            &saved("{id:\"minecraft:pig\",Age:0}"),
            ZOMBIFIED_PIGLIN,
            None,
        );
        assert_eq!(flag(&adult, "IsBaby"), Some(0));
    }

    #[test]
    fn a_turned_villager_drops_its_trades_hand_and_loot_pickup() {
        let villager = saved(
            "{id:\"minecraft:villager\",Pos:[1.5d,64.0d,2.5d],Age:-24000,CanPickUpLoot:1b,VillagerData:{type:\"minecraft:plains\",profession:\"minecraft:farmer\",level:2},Xp:10,Gossips:[],Offers:{Recipes:[]},equipment:{mainhand:{id:\"minecraft:wheat\",count:3}},Silent:1b}",
        );
        let witch = turned_tag(&villager, WITCH, None);
        assert_eq!(witch.get("id").and_then(Tag::as_str), Some(WITCH));
        for key in [
            "CanPickUpLoot",
            "VillagerData",
            "Xp",
            "Gossips",
            "Offers",
            "equipment",
            "IsBaby",
            "Age",
        ] {
            assert_eq!(witch.get(key), None, "{key}");
        }
        assert_eq!(flag(&witch, "PersistenceRequired"), Some(1));
        assert_eq!(flag(&witch, "Silent"), Some(1));
        assert_eq!(witch.get("Pos"), villager.get("Pos"));
    }

    #[test]
    fn a_friend_sees_the_brown_mooshroom_and_the_charged_creeper() {
        use crate::mob_tracking::{LOOK_TICKS, MobEvent, Trackers, Watcher};
        let mut host = EntityWorld::default();
        host.set_next_entity_id(600);
        let mooshroom_tag =
            saved("{id:\"minecraft:mooshroom\",Pos:[2.5d,64.0d,0.5d],Type:\"red\"}");
        let creeper_tag = saved("{id:\"minecraft:creeper\",Pos:[3.5d,64.0d,0.5d]}");
        let mooshroom = crate::server_mobs::spawn_saved(&mut host, &mooshroom_tag).unwrap();
        let creeper = crate::server_mobs::spawn_saved(&mut host, &creeper_tag).unwrap();
        let originals: HashMap<u64, Tag> = [(mooshroom, mooshroom_tag), (creeper, creeper_tag)]
            .into_iter()
            .collect();
        let watcher = Watcher {
            id: 3,
            feet: [0.5, 64.0, 0.5],
            view_distance: 8,
        };
        let mut trackers = Trackers::default();
        trackers.tick();
        let mut friend = EntityWorld::default();
        for event in trackers.update(&host, &originals, &[watcher]).events_of(3) {
            let MobEvent::Add(id, sent) = event else {
                panic!("{event:?}")
            };
            assert!(crate::mob_tracking::spawn_tracked(&mut friend, *id, sent));
        }
        let bolt = Bolt {
            uuid: [1, 2, 3, 4],
            at: DVec3::new(2.5, 64.0, 0.5),
        };
        assert!(host.thunder_hit(&bolt, &Air).is_empty());
        let mut looks = Vec::new();
        for _ in 0..LOOK_TICKS {
            trackers.tick();
            looks.extend(trackers.update(&host, &originals, &[watcher]).events);
        }
        assert_eq!(looks.len(), 2, "{looks:?}");
        for (_, event) in &looks {
            let MobEvent::Look(id, sent) = event else {
                panic!("{event:?}")
            };
            assert!(crate::mob_tracking::spawn_tracked(&mut friend, *id, sent));
        }
        let shown = friend.cow_mut(mooshroom).unwrap();
        let variant = shown.mooshroom.as_ref().unwrap().variant;
        assert_eq!(
            variant,
            minecraftoss_entities::mooshroom::MushroomVariant::Brown
        );
        assert!(friend.creeper_mut(creeper).unwrap().creeper.powered);
    }
}
