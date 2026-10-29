use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::mob_actions::{Actor, MobOutcome, MobSound};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::age::Age;
use minecraftoss_entities::animal::AnimalEvent;
use minecraftoss_entities::cow::InteractionResult;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::land::rabbits::Rabbit;
use minecraftoss_entities::world::families::land::rabbits::{self, RabbitVariant};

mod render;

pub const SHADOW_RADIUS: f32 = 0.3;
const HOPPING: u32 = 1;

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn custom_name(tag: &Tag) -> Option<String> {
    match tag.get("CustomName")? {
        Tag::String(name) => Some(name.clone()),
        name => name.get("text").and_then(Tag::as_str).map(str::to_owned),
    }
}

impl TerrainFamily for Rabbit {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["MoreCarrotTicks", "ForcedAge"];
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let yaw = mob.core.yaw;
        let ai = &mut mob.state.ai;
        ai.yaw = yaw;
        ai.body_rotation.body_yaw = yaw;
        ai.state.look_control.head_yaw = yaw;
        let variant = RabbitVariant::from_id(int(tag, "RabbitType").unwrap_or(0));
        mob.state.set_variant(variant);
        mob.state.goals_mut().more_carrot_ticks = int(tag, "MoreCarrotTicks").unwrap_or(0);
        let shown_baby = int(tag, "Baby").is_some_and(|baby| baby != 0);
        let ticks = int(tag, "Age").unwrap_or(if shown_baby { Age::BABY_START } else { 0 });
        mob.state.age = Age {
            ticks,
            forced: int(tag, "ForcedAge").unwrap_or(0),
            locked: int(tag, "AgeLocked").is_some_and(|locked| locked != 0),
            ..Age::default()
        };
        mob.state.in_love = int(tag, "InLove").unwrap_or(0);
        mob.state.named = tag.get("CustomName").is_some();
        mob.state.toast = custom_name(tag).as_deref() == Some(rabbits::TOAST);
        let baby = mob.state.age.baby();
        rabbits::set_baby(mob, baby);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let state = &mob.state;
        let carrots = state.goals().map_or(0, |goals| goals.more_carrot_ticks);
        let mut fields = vec![
            ("RabbitType", Tag::Int(state.variant.id())),
            ("MoreCarrotTicks", Tag::Int(carrots)),
            ("Age", Tag::Int(state.age.ticks)),
            ("ForcedAge", Tag::Int(state.age.forced)),
            ("AgeLocked", Tag::Byte(i8::from(state.age.locked))),
            ("InLove", Tag::Int(state.in_love)),
        ];
        if state.variant == RabbitVariant::Evil && !state.named {
            let name = Tag::String(rabbits::KILLER_BUNNY.to_owned());
            let name = [("translate".to_owned(), name)].into_iter().collect();
            fields.push(("CustomName", Tag::Compound(name)));
        }
        fields
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let jump = mob.state.jump();
        let mut show = FamilyShow::default();
        if jump.duration > 0 {
            show.flags |= HOPPING;
            show.values[0] = jump.ticks;
        }
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        let jump = &mut mob.state.goals_mut().jump;
        if show.flags & HOPPING != 0 {
            jump.duration = rabbits::JUMP_DURATION;
            jump.ticks = show.values[0];
        } else {
            jump.duration = 0;
            jump.ticks = 0;
        }
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
        render::append_rabbits(out.culled, shown, poses, atlas, light, partial);
    }

    fn interact(
        world: &mut EntityWorld,
        id: u64,
        actor: &mut Actor,
        outcome: &mut MobOutcome,
    ) -> bool {
        let fed = rabbits::feed(world, id, actor.inventory, actor.selected, actor.infinite);
        let Some((result, events, position)) = fed else {
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
    use glam::DVec3;
    use minecraftoss_entities::world::MobHit;
    use minecraftoss_player::inventory::{Inventory, ItemStack};
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn rabbit(world: &EntityWorld, id: u64) -> &FamilyMob<Rabbit> {
        world
            .family::<Rabbit>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    #[test]
    fn a_rabbit_saves_and_loads_with_its_type_carrot_ticks_and_age() {
        for (saved, variant) in [
            (0, RabbitVariant::Brown),
            (1, RabbitVariant::White),
            (2, RabbitVariant::Black),
            (3, RabbitVariant::WhiteSplotched),
            (4, RabbitVariant::Gold),
            (5, RabbitVariant::Salt),
            (99, RabbitVariant::Evil),
            (7, RabbitVariant::Brown),
        ] {
            let mut world = EntityWorld::default();
            let tag = parse(&format!(
                "{{id:\"minecraft:rabbit\",Pos:[1.5d,64.0d,2.5d],RabbitType:{saved}}}"
            ));
            let id = spawn_saved(&mut world, &tag).expect("a rabbit loads");
            assert_eq!(rabbit(&world, id).state.variant, variant, "{saved}");
            let goals = rabbit(&world, id).state.goals().unwrap();
            assert_eq!(goals.evil, variant == RabbitVariant::Evil, "{saved}");
            let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
            let saved = mob_tags(&world, |_| true, &originals).remove(0).1;
            let name = saved.get("CustomName");
            if variant == RabbitVariant::Evil {
                let translate = name.and_then(|name| name.get("translate"));
                assert_eq!(
                    translate.and_then(Tag::as_str),
                    Some("entity.minecraft.killer_bunny"),
                    "an unnamed killer bunny is The Killer Bunny"
                );
                let mut again = EntityWorld::default();
                let back = spawn_saved(&mut again, &saved).unwrap();
                assert!(rabbit(&again, back).state.named);
                let loaded: HashMap<u64, Tag> = [(back, saved.clone())].into_iter().collect();
                let resaved = mob_tags(&again, |_| true, &loaded).remove(0).1;
                assert_eq!(resaved.get("CustomName"), name);
            } else {
                assert_eq!(name, None, "{variant:?}");
            }
        }
        for (name, toast) in [
            ("\"Toast\"", true),
            ("{text:\"Toast\"}", true),
            ("\"toast\"", false),
            ("\"Bob\"", false),
        ] {
            let mut world = EntityWorld::default();
            let tag = parse(&format!(
                "{{id:\"minecraft:rabbit\",Pos:[1.5d,64.0d,2.5d],RabbitType:99,CustomName:{name}}}"
            ));
            let id = spawn_saved(&mut world, &tag).unwrap();
            let state = &rabbit(&world, id).state;
            assert_eq!((state.named, state.toast), (true, toast), "{name}");
            let originals: HashMap<u64, Tag> = [(id, tag.clone())].into_iter().collect();
            let saved = mob_tags(&world, |_| true, &originals).remove(0).1;
            assert_eq!(
                saved.get("CustomName"),
                tag.get("CustomName"),
                "a name is kept"
            );
        }
        let text = "{id:\"minecraft:rabbit\",Pos:[1.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],RabbitType:3,MoreCarrotTicks:25,Age:-1200,ForcedAge:40,AgeLocked:1b,InLove:0,Health:2.0f,PersistenceRequired:1b}";
        let tag = parse(text);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).unwrap();
        let mob = rabbit(&world, id);
        assert_eq!(world.entity_type(id), Some(rabbits::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(48));
        assert_eq!(mob.state.variant, RabbitVariant::WhiteSplotched);
        assert_eq!(mob.state.goals().unwrap().more_carrot_ticks, 25);
        assert_eq!(
            (
                mob.state.age.ticks,
                mob.state.age.forced,
                mob.state.age.locked
            ),
            (-1200, 40, true)
        );
        assert!(mob.core.baby && mob.core.persistent);
        assert_eq!((mob.core.body.width, mob.core.health), (0.24, 2.0));
        assert_eq!(
            (mob.state.ai.yaw, mob.state.ai.body_rotation.body_yaw),
            (90.0, 90.0)
        );
        world.family_mob_mut::<Rabbit>(id).unwrap().state.in_love = 300;
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = mob_tags(&world, |_| true, &originals);
        let saved = &saved.iter().find(|(saved, _)| *saved == id).unwrap().1;
        for (key, value) in [
            ("RabbitType", Tag::Int(3)),
            ("MoreCarrotTicks", Tag::Int(25)),
            ("Age", Tag::Int(-1200)),
            ("ForcedAge", Tag::Int(40)),
            ("AgeLocked", Tag::Byte(1)),
            ("InLove", Tag::Int(300)),
        ] {
            assert_eq!(saved.get(key), Some(&value), "{key}");
        }
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, saved).unwrap();
        let back = rabbit(&again, back);
        assert_eq!(
            (back.state.variant, back.state.in_love),
            (RabbitVariant::WhiteSplotched, 300)
        );
        assert!(back.core.baby && back.state.age.locked);
    }

    #[test]
    fn a_friend_sees_a_rabbit_from_its_look_tag() {
        let mut host = EntityWorld::default();
        let tag = parse(
            "{id:\"minecraft:rabbit\",Pos:[1.5d,64.0d,2.5d],RabbitType:4,Age:-500,MoreCarrotTicks:30,ForcedAge:20}",
        );
        let id = spawn_saved(&mut host, &tag).unwrap();
        let originals: HashMap<u64, Tag> = [(id, tag)].into_iter().collect();
        let saved = mob_tags(&host, |_| true, &originals).remove(0).1;
        let seen = look(&saved);
        for key in ["MoreCarrotTicks", "ForcedAge", "Age", "InLove"] {
            assert_eq!(seen.get(key), None, "{key} changes while it lives");
        }
        assert_eq!(
            seen.get("Baby"),
            Some(&Tag::Byte(1)),
            "growing up is a new look"
        );
        assert_eq!(seen.get("RabbitType"), Some(&Tag::Int(4)));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mirror = rabbit(&friend, id);
        assert_eq!(mirror.state.variant, RabbitVariant::Gold);
        assert!(mirror.core.baby);
        assert_eq!(mirror.core.body.height, 0.4);
        assert_eq!(crate::families::tracking_range(rabbits::KIND), Some(8));
        assert_eq!(shows(&host)[&id].family, Some(FamilyShow::default()));
        let jump = &mut host
            .family_mob_mut::<Rabbit>(id)
            .unwrap()
            .state
            .goals_mut()
            .jump;
        jump.duration = rabbits::JUMP_DURATION;
        jump.ticks = 4;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!((show.flags, show.values[0]), (HOPPING, 4));
        let mirror = friend.family_mob_mut::<Rabbit>(id).unwrap();
        Rabbit::show_on(mirror, &show);
        assert_eq!(
            (mirror.state.jump().duration, mirror.state.jump().ticks),
            (15, 4),
            "the friend sees it hop"
        );
        Rabbit::show_on(mirror, &FamilyShow::default());
        assert_eq!(mirror.state.jump().duration, 0);
    }

    #[test]
    fn a_rabbit_eats_a_carrot_from_the_hand_and_falls_in_love() {
        let mut world = EntityWorld::default();
        let tag = parse("{id:\"minecraft:rabbit\",Pos:[1.5d,64.0d,2.5d]}");
        let id = spawn_saved(&mut world, &tag).unwrap();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:carrot", 2));
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        let outcome = interact(&mut world, MobHit::Family(48, id), &mut actor);
        assert!(outcome.handled && outcome.inventory_changed);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(rabbit(&world, id).state.in_love, 600);
        assert_eq!(world.take_entity_events(), vec![(id, 18)], "hearts");
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        let again = interact(&mut world, MobHit::Family(48, id), &mut actor);
        assert!(!again.handled, "a rabbit in love eats no more");
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        inventory.slots[0] = Some(ItemStack::new("minecraft:wheat", 1));
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        assert!(!interact(&mut world, MobHit::Family(48, id), &mut actor).handled);
        let position = rabbit(&world, id).core.body.position;
        assert_eq!(position, DVec3::new(1.5, 64.0, 2.5));
    }
}
