use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::mob_actions::{Actor, MobOutcome, MobSound};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::age::Age;
use minecraftoss_entities::animal::AnimalEvent;
use minecraftoss_entities::cow::InteractionResult;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::families::land::polar_bears;
pub use minecraftoss_entities::world::families::land::polar_bears::PolarBear;

mod render;

pub const SHADOW_RADIUS: f32 = 0.9;
const STANDING: u32 = 1;

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn uuid(tag: &Tag, key: &str) -> Option<u128> {
    let ints = tag.get(key).and_then(Tag::as_ints)?;
    <[i32; 4]>::try_from(ints).ok().map(uuid_from_ints)
}

impl TerrainFamily for PolarBear {
    const TRACKING_RANGE: i32 = 10;
    const VOLATILE: &'static [&'static str] = &["angry_at", "ForcedAge"];
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;
    const CAGE_MARGIN: f32 = 0.13;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let yaw = mob.core.yaw;
        let ai = &mut mob.state.ai;
        ai.yaw = yaw;
        ai.body_rotation.body_yaw = yaw;
        ai.state.look_control.head_yaw = yaw;
        let end = tag.get("anger_end_time").and_then(Tag::as_i64);
        ai.state.enderman.anger_end_time = end.unwrap_or(-1);
        mob.state.angry_at = uuid(tag, "angry_at");
        let shown_baby = int(tag, "Baby").is_some_and(|baby| baby != 0);
        let ticks = int(tag, "Age").unwrap_or(if shown_baby { Age::BABY_START } else { 0 });
        mob.state.age = Age {
            ticks,
            forced: int(tag, "ForcedAge").unwrap_or(0),
            locked: int(tag, "AgeLocked").is_some_and(|locked| locked != 0),
            ..Age::default()
        };
        mob.state.in_love = int(tag, "InLove").unwrap_or(0);
        let baby = mob.state.age.baby();
        polar_bears::set_baby(mob, baby);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let state = &mob.state;
        let mut fields = vec![
            ("Age", Tag::Int(state.age.ticks)),
            ("ForcedAge", Tag::Int(state.age.forced)),
            ("AgeLocked", Tag::Byte(i8::from(state.age.locked))),
            ("InLove", Tag::Int(state.in_love)),
            ("anger_end_time", Tag::Long(state.anger_end_time())),
        ];
        if let Some(uuid) = state.angry_at {
            fields.push(("angry_at", Tag::IntArray(uuid_to_ints(uuid).to_vec())));
        }
        fields
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let state = &mob.state;
        let mut show = FamilyShow::default();
        if state.standing() {
            show.flags |= STANDING;
        }
        show.values[0] = state.stand;
        show.values[1] = state.stand_old;
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        let state = &mut mob.state;
        state.goals_mut().standing = show.flags & STANDING != 0;
        state.stand = show.values[0];
        state.stand_old = show.values[1];
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
        render::append_polar_bears(out.culled, shown, poses, atlas, light, partial);
    }

    fn interact(
        world: &mut EntityWorld,
        id: u64,
        actor: &mut Actor,
        outcome: &mut MobOutcome,
    ) -> bool {
        let used =
            polar_bears::interact(world, id, actor.inventory, actor.selected, actor.infinite);
        let Some((result, events, position)) = used else {
            return false;
        };
        if result == InteractionResult::Pass {
            return false;
        }
        outcome.inventory_changed = true;
        let lock = events.iter().find_map(|event| match event {
            AnimalEvent::AgeLock => Some("item.golden_dandelion.use"),
            AnimalEvent::AgeUnlock => Some("item.golden_dandelion.unuse"),
            AnimalEvent::Hearts => None,
        });
        if let Some(event) = lock {
            outcome.sounds.push(MobSound {
                event: event.to_owned(),
                position,
                volume: 1.0,
                pitch: 1.0,
                category: "players_volume",
            });
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob_actions::interact;
    use crate::mob_tracking::{look, shows, spawn_tracked};
    use crate::server_mobs::{mob_tags, spawn_saved};
    use minecraftoss_entities::world::MobHit;
    use minecraftoss_player::inventory::{Inventory, ItemStack};
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn bear(world: &EntityWorld, id: u64) -> &FamilyMob<PolarBear> {
        world
            .family::<PolarBear>()
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

    #[test]
    fn a_polar_bear_saves_and_loads_with_its_anger_and_age() {
        let text = "{id:\"minecraft:polar_bear\",Pos:[1.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],Age:-1200,ForcedAge:40,AgeLocked:1b,InLove:0,Health:20.0f,anger_end_time:5000L,angry_at:[I;1,2,3,4],PersistenceRequired:1b}";
        let tag = parse(text);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a polar bear loads");
        assert_eq!(world.entity_type(id), Some(polar_bears::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(55));
        let mob = bear(&world, id);
        assert_eq!(
            (
                mob.state.age.ticks,
                mob.state.age.forced,
                mob.state.age.locked
            ),
            (-1200, 40, true)
        );
        assert!(mob.core.baby && mob.core.persistent);
        assert_eq!(
            (mob.core.body.width, mob.core.body.height, mob.core.health),
            (0.7, 0.7, 20.0)
        );
        assert_eq!(mob.state.anger_end_time(), 5000);
        assert_eq!(mob.state.angry_at, Some(uuid_from_ints([1, 2, 3, 4])));
        assert_eq!(
            (mob.state.ai.yaw, mob.state.ai.body_rotation.body_yaw),
            (90.0, 90.0)
        );
        let saved = saved_tag(&world, id, tag);
        for (key, value) in [
            ("Age", Tag::Int(-1200)),
            ("ForcedAge", Tag::Int(40)),
            ("AgeLocked", Tag::Byte(1)),
            ("InLove", Tag::Int(0)),
            ("anger_end_time", Tag::Long(5000)),
            ("angry_at", Tag::IntArray(vec![1, 2, 3, 4])),
        ] {
            assert_eq!(saved.get(key), Some(&value), "{key}");
        }
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        let back = bear(&again, back);
        assert_eq!(back.state.anger_end_time(), 5000);
        assert_eq!(back.state.angry_at, Some(uuid_from_ints([1, 2, 3, 4])));
        assert!(back.core.baby && back.state.age.locked);

        let calm = parse("{id:\"minecraft:polar_bear\",Pos:[1.5d,64.0d,2.5d]}");
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &calm).unwrap();
        let mob = bear(&world, id);
        assert!(!mob.core.baby);
        assert_eq!((mob.core.body.width, mob.core.health), (1.4, 30.0));
        assert_eq!((mob.state.anger_end_time(), mob.state.angry_at), (-1, None));
        let saved = saved_tag(&world, id, calm);
        assert_eq!(saved.get("anger_end_time"), Some(&Tag::Long(-1)));
        assert_eq!(saved.get("angry_at"), None, "no anger, no target saved");
    }

    #[test]
    fn a_friend_sees_a_polar_bear_stand_up_from_its_look_tag() {
        let mut host = EntityWorld::default();
        let tag = parse(
            "{id:\"minecraft:polar_bear\",Pos:[1.5d,64.0d,2.5d],Age:-500,ForcedAge:20,anger_end_time:900L,angry_at:[I;1,2,3,4]}",
        );
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, tag);
        let seen = look(&saved);
        for key in ["angry_at", "ForcedAge", "Age", "InLove", "anger_end_time"] {
            assert_eq!(seen.get(key), None, "{key} changes while it lives");
        }
        assert_eq!(seen.get("Baby"), Some(&Tag::Byte(1)));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mirror = bear(&friend, id);
        assert!(mirror.core.baby);
        assert_eq!(mirror.core.body.height, 0.7);
        assert_eq!(crate::families::tracking_range(polar_bears::KIND), Some(10));
        assert_eq!(shows(&host)[&id].family, Some(FamilyShow::default()));
        let mob = host.family_mob_mut::<PolarBear>(id).unwrap();
        mob.state.goals_mut().standing = true;
        mob.state.stand = 3;
        mob.state.stand_old = 2;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(
            (show.flags, show.values[0], show.values[1]),
            (STANDING, 3, 2),
            "standing, three ticks up"
        );
        PolarBear::show_on(friend.family_mob_mut::<PolarBear>(id).unwrap(), &show);
        let mirror = &bear(&friend, id).state;
        assert!(mirror.standing());
        assert_eq!((mirror.stand, mirror.stand_old), (3, 2));
        PolarBear::show_on(
            friend.family_mob_mut::<PolarBear>(id).unwrap(),
            &FamilyShow::default(),
        );
        let mirror = &bear(&friend, id).state;
        assert!(!mirror.standing());
        assert_eq!((mirror.stand, mirror.stand_old), (0, 0));
    }

    #[test]
    fn a_golden_dandelion_locks_a_cub_s_age_and_nothing_feeds_a_polar_bear() {
        let mut world = EntityWorld::default();
        let cub = parse("{id:\"minecraft:polar_bear\",Pos:[1.5d,64.0d,2.5d],Age:-500}");
        let id = spawn_saved(&mut world, &cub).unwrap();
        let mut inventory = Inventory::default();
        for item in ["minecraft:cod", "minecraft:salmon", "minecraft:wheat"] {
            inventory.slots[0] = Some(ItemStack::new(item, 2));
            let mut actor = Actor {
                player: 1,
                inventory: &mut inventory,
                selected: 0,
                infinite: false,
                entity_loot: None,
                shearing_loot: None,
            };
            let outcome = interact(&mut world, MobHit::Family(55, id), &mut actor);
            assert!(!outcome.handled, "{item}");
        }
        inventory.slots[0] = Some(ItemStack::new("minecraft:golden_dandelion", 2));
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        let outcome = interact(&mut world, MobHit::Family(55, id), &mut actor);
        assert!(outcome.handled && outcome.inventory_changed);
        assert_eq!(outcome.sounds[0].event, "item.golden_dandelion.use");
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        let mob = bear(&world, id);
        assert!(mob.state.age.locked && mob.core.persistent);
        assert_eq!(world.take_entity_events(), Vec::new(), "no hearts");
    }
}
