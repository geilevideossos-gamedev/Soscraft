use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, FamilyShow, TerrainFamily};
use crate::item_nbt::stack_tag;
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::mob_actions::{Actor, MobOutcome, MobSound};
use crate::piglin_save::stack_of;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::age::Age;
use minecraftoss_entities::animal::AnimalEvent;
use minecraftoss_entities::cow::InteractionResult;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::land::pandas::Panda;
use minecraftoss_entities::world::families::land::pandas::{self, Gene};
use minecraftoss_player::inventory::ItemStack;
use std::collections::BTreeMap;

pub const SHADOW_RADIUS: f32 = 0.9;
pub const SITTING: u32 = 1;
pub const ON_BACK: u32 = 2;
pub const ROLLING: u32 = 4;
pub const SNEEZING: u32 = 8;
pub const SCARED: u32 = 16;
pub const UNHAPPY_VALUE: usize = 2;
pub const HELD_VALUE: usize = 4;
pub const HELD_NOTHING: i32 = 0;
pub const HELD_BAMBOO: i32 = 1;
pub const HELD_CAKE: i32 = 2;
pub const HELD_OTHER: i32 = 3;
const BAMBOO: &str = "minecraft:bamboo";
const CAKE: &str = "minecraft:cake";
const MAX_HEALTH: &str = "minecraft:max_health";
const MOVEMENT_SPEED: &str = "minecraft:movement_speed";
const MAINHAND: &str = "mainhand";

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn attribute_base(tag: &Tag, id: &str) -> Option<f64> {
    let list = tag.get("attributes")?.as_list()?;
    let entry = list
        .iter()
        .find(|entry| entry.get("id").and_then(Tag::as_str) == Some(id))?;
    entry.get("base")?.as_f64()
}

fn attribute(id: &str, base: f64) -> Tag {
    let entry = [
        ("id".to_owned(), Tag::String(id.to_owned())),
        ("base".to_owned(), Tag::Double(base)),
    ];
    Tag::Compound(entry.into_iter().collect())
}

fn gene(tag: &Tag, key: &str) -> Gene {
    Gene::from_name(tag.get(key).and_then(Tag::as_str).unwrap_or_default())
}

pub fn held_value(held: Option<&ItemStack>) -> i32 {
    match held.map(|stack| stack.id.as_str()) {
        None => HELD_NOTHING,
        Some(BAMBOO) => HELD_BAMBOO,
        Some(CAKE) => HELD_CAKE,
        Some(_) => HELD_OTHER,
    }
}

fn held_of(value: i32) -> Option<ItemStack> {
    match value {
        HELD_BAMBOO => Some(ItemStack::new(BAMBOO, 1)),
        HELD_CAKE => Some(ItemStack::new(CAKE, 1)),
        _ => None,
    }
}

impl TerrainFamily for Panda {
    const TRACKING_RANGE: i32 = 10;
    const VOLATILE: &'static [&'static str] = &["equipment", "drop_chances", "ForcedAge"];
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let yaw = mob.core.yaw;
        let ai = &mut mob.state.ai;
        ai.yaw = yaw;
        ai.body_rotation.body_yaw = yaw;
        ai.state.look_control.head_yaw = yaw;
        let state = &mut mob.state;
        state.set_genes(gene(tag, "MainGene"), gene(tag, "HiddenGene"));
        let shown_baby = int(tag, "Baby").is_some_and(|baby| baby != 0);
        let ticks = int(tag, "Age").unwrap_or(if shown_baby { Age::BABY_START } else { 0 });
        state.age = Age {
            ticks,
            forced: int(tag, "ForcedAge").unwrap_or(0),
            locked: int(tag, "AgeLocked").is_some_and(|locked| locked != 0),
            ..Age::default()
        };
        state.in_love = int(tag, "InLove").unwrap_or(0);
        state.can_pick_up_loot = int(tag, "CanPickUpLoot").is_none_or(|can| can != 0);
        state.held = tag
            .get("equipment")
            .and_then(|equipment| equipment.get(MAINHAND))
            .and_then(stack_of);
        state.drop_chance = tag
            .get("drop_chances")
            .and_then(|chances| chances.get(MAINHAND))
            .and_then(Tag::as_f64)
            .map_or(pandas::DROP_CHANCE, |chance| chance as f32);
        if let Some(speed) = attribute_base(tag, MOVEMENT_SPEED) {
            state.speed = speed;
        }
        if let Some(max_health) = attribute_base(tag, MAX_HEALTH) {
            let core = &mut mob.core;
            core.max_health = max_health as f32;
            core.health = core.health.min(core.max_health);
        }
        let baby = mob.state.age.baby();
        pandas::set_baby(mob, baby);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let state = &mob.state;
        let mut equipment = BTreeMap::new();
        if let Some(stack) = state.held.as_ref().and_then(stack_tag) {
            equipment.insert(MAINHAND.to_owned(), stack);
        }
        let mut chances = BTreeMap::new();
        if state.drop_chance != pandas::DROP_CHANCE {
            chances.insert(MAINHAND.to_owned(), Tag::Float(state.drop_chance));
        }
        let mut fields = vec![
            ("MainGene", Tag::String(state.main_gene.name().to_owned())),
            (
                "HiddenGene",
                Tag::String(state.hidden_gene.name().to_owned()),
            ),
            ("Age", Tag::Int(state.age.ticks)),
            ("ForcedAge", Tag::Int(state.age.forced)),
            ("AgeLocked", Tag::Byte(i8::from(state.age.locked))),
            ("InLove", Tag::Int(state.in_love)),
            ("CanPickUpLoot", Tag::Byte(i8::from(state.can_pick_up_loot))),
            ("equipment", Tag::Compound(equipment)),
            ("drop_chances", Tag::Compound(chances)),
        ];
        if state.bred {
            let max_health = attribute(MAX_HEALTH, f64::from(mob.core.max_health));
            let speed = attribute(MOVEMENT_SPEED, state.speed);
            fields.push(("attributes", Tag::List(vec![max_health, speed])));
        }
        fields
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let mut show = FamilyShow::default();
        if let Some(goals) = mob.state.goals() {
            for (set, flag) in [
                (goals.sitting, SITTING),
                (goals.on_back, ON_BACK),
                (goals.rolling, ROLLING),
                (goals.sneezing, SNEEZING),
                (goals.scared, SCARED),
            ] {
                if set {
                    show.flags |= flag;
                }
            }
            show.values[UNHAPPY_VALUE] = goals.unhappy;
        }
        show.values[HELD_VALUE] = held_value(mob.state.held.as_ref());
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        let state = &mut mob.state;
        state.held = held_of(show.values[HELD_VALUE]);
        let goals = state.goals_mut();
        goals.sitting = show.flags & SITTING != 0;
        goals.on_back = show.flags & ON_BACK != 0;
        goals.rolling = show.flags & ROLLING != 0;
        goals.sneezing = show.flags & SNEEZING != 0;
        goals.scared = show.flags & SCARED != 0;
        goals.unhappy = show.values[UNHAPPY_VALUE];
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

    fn interact(
        world: &mut EntityWorld,
        id: u64,
        actor: &mut Actor,
        outcome: &mut MobOutcome,
    ) -> bool {
        let used = pandas::interact(world, id, actor.inventory, actor.selected, actor.infinite);
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
    use glam::DVec3;
    use minecraftoss_entities::world::MobHit;
    use minecraftoss_entities::world::families::Family;
    use minecraftoss_player::inventory::Inventory;
    use std::collections::HashMap;

    fn parse(text: &str) -> Tag {
        minecraftoss_core::snbt::parse_compound(text).unwrap()
    }

    fn panda(world: &EntityWorld, id: u64) -> &FamilyMob<Panda> {
        world
            .family::<Panda>()
            .iter()
            .find(|mob| mob.core.id == id)
            .unwrap()
    }

    fn saved_tag(world: &EntityWorld, id: u64, tag: Option<Tag>) -> Tag {
        let originals: HashMap<u64, Tag> = tag.map(|tag| (id, tag)).into_iter().collect();
        let mut saved = mob_tags(world, |_| true, &originals);
        saved.retain(|(saved, _)| *saved == id);
        saved.remove(0).1
    }

    fn mainhand(tag: &Tag) -> Option<&Tag> {
        tag.get("equipment")?.get(MAINHAND)
    }

    #[test]
    fn a_panda_saves_and_loads_with_its_genes_held_item_love_and_attributes() {
        let text = "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d],Rotation:[90.0f,0.0f],MainGene:\"weak\",HiddenGene:\"weak\",Age:-1200,ForcedAge:40,AgeLocked:1b,InLove:0,CanPickUpLoot:0b,Health:8.0f,equipment:{mainhand:{id:\"minecraft:bamboo\",count:3}},drop_chances:{mainhand:2.0f},attributes:[{id:\"minecraft:max_health\",base:10.0d},{id:\"minecraft:movement_speed\",base:0.07d}],PersistenceRequired:1b}";
        let tag = parse(text);
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).expect("a panda loads");
        assert_eq!(world.entity_type(id), Some(pandas::KIND));
        assert_eq!(world.family_core(id).map(|(key, _)| key), Some(52));
        let mob = panda(&world, id);
        assert_eq!(
            (
                mob.state.main_gene,
                mob.state.hidden_gene,
                mob.state.variant()
            ),
            (Gene::Weak, Gene::Weak, Gene::Weak)
        );
        assert_eq!(mob.state.goals().unwrap().variant, Gene::Weak);
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
            (mob.core.body.width, mob.core.health, mob.core.max_health),
            (0.65, 8.0, 10.0)
        );
        assert_eq!(mob.state.speed, 0.07);
        assert!(!mob.state.can_pick_up_loot);
        let held = mob.state.held.as_ref().unwrap();
        assert_eq!((held.id.as_str(), held.count), ("minecraft:bamboo", 3));
        assert_eq!(mob.state.drop_chance, 2.0);
        assert_eq!(
            (mob.state.ai.yaw, mob.state.ai.body_rotation.body_yaw),
            (90.0, 90.0)
        );
        let saved = saved_tag(&world, id, Some(tag.clone()));
        for (key, value) in [
            ("MainGene", Tag::String("weak".to_owned())),
            ("HiddenGene", Tag::String("weak".to_owned())),
            ("Age", Tag::Int(-1200)),
            ("ForcedAge", Tag::Int(40)),
            ("AgeLocked", Tag::Byte(1)),
            ("InLove", Tag::Int(0)),
            ("CanPickUpLoot", Tag::Byte(0)),
        ] {
            assert_eq!(saved.get(key), Some(&value), "{key}");
        }
        let stack = mainhand(&saved).unwrap();
        assert_eq!(
            stack.get("id").and_then(Tag::as_str),
            Some("minecraft:bamboo")
        );
        assert_eq!(stack.get("count").and_then(Tag::as_i64), Some(3));
        let chance = saved.get("drop_chances").and_then(|c| c.get(MAINHAND));
        assert_eq!(chance.and_then(Tag::as_f64), Some(2.0));
        assert_eq!(
            saved.get("attributes"),
            tag.get("attributes"),
            "kept as loaded"
        );
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        let back = panda(&again, back);
        assert_eq!(back.state.variant(), Gene::Weak);
        assert_eq!((back.core.max_health, back.state.speed), (10.0, 0.07));
        assert!(back.core.baby && back.state.age.locked);
        assert_eq!(back.state.held.as_ref().map(|s| s.count), Some(3));

        for (text, main, hidden) in [
            (
                "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d]}",
                Gene::Normal,
                Gene::Normal,
            ),
            (
                "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d],MainGene:\"sleepy\",HiddenGene:\"brown\"}",
                Gene::Normal,
                Gene::Brown,
            ),
        ] {
            let tag = parse(text);
            let mut world = EntityWorld::default();
            let id = spawn_saved(&mut world, &tag).unwrap();
            let mob = panda(&world, id);
            assert_eq!((mob.state.main_gene, mob.state.hidden_gene), (main, hidden));
            assert!(mob.state.can_pick_up_loot, "the constructor's default");
            assert_eq!(
                (mob.state.held.as_ref(), mob.state.drop_chance),
                (None, 0.085)
            );
            assert_eq!(
                (mob.core.max_health, mob.state.speed),
                (20.0, pandas::SPEED)
            );
            let saved = saved_tag(&world, id, Some(tag));
            assert_eq!(mainhand(&saved), None);
            assert_eq!(
                saved.get("drop_chances"),
                Some(&Tag::Compound(BTreeMap::new()))
            );
            assert_eq!(saved.get("attributes"), None, "no attributes written");
            assert_eq!(
                saved.get("MainGene"),
                Some(&Tag::String(main.name().to_owned()))
            );
        }

        let mut world = EntityWorld::default();
        let mut cub = Panda::spawn(pandas::KIND, DVec3::new(0.5, 64.0, 0.5)).unwrap();
        cub.state.age.set(Age::BABY_START);
        pandas::set_baby(&mut cub, true);
        cub.state.set_genes(Gene::Weak, Gene::Weak);
        pandas::set_attributes(&mut cub);
        cub.state.bred = true;
        let id = world.spawn_family(cub);
        let saved = saved_tag(&world, id, None);
        let attributes = saved.get("attributes").and_then(Tag::as_list).unwrap();
        assert_eq!(attributes.len(), 2);
        assert_eq!(attribute_base(&saved, MAX_HEALTH), Some(10.0));
        assert_eq!(attribute_base(&saved, MOVEMENT_SPEED), Some(pandas::SPEED));
        let mut again = EntityWorld::default();
        let back = spawn_saved(&mut again, &saved).unwrap();
        let back = panda(&again, back);
        assert_eq!((back.core.max_health, back.core.health), (10.0, 10.0));
        assert!(back.core.baby);
    }

    #[test]
    fn a_summoned_weak_panda_keeps_twenty_health() {
        let tag = parse(
            "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d],MainGene:\"weak\",HiddenGene:\"weak\"}",
        );
        let mut world = EntityWorld::default();
        let id = spawn_saved(&mut world, &tag).unwrap();
        let mob = panda(&world, id);
        assert_eq!(mob.state.variant(), Gene::Weak);
        assert_eq!(
            (mob.core.health, mob.core.max_health, mob.state.speed),
            (20.0, 20.0, pandas::SPEED),
            "attributes change only on spawning and breeding"
        );
        let tag = parse(
            "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d],MainGene:\"lazy\",HiddenGene:\"normal\"}",
        );
        let id = spawn_saved(&mut world, &tag).unwrap();
        assert_eq!(panda(&world, id).state.speed, pandas::SPEED);
    }

    #[test]
    fn a_friend_sees_a_panda_from_its_look_tag() {
        let mut host = EntityWorld::default();
        let tag = parse(
            "{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d],MainGene:\"lazy\",HiddenGene:\"worried\",Age:-500,ForcedAge:20,equipment:{mainhand:{id:\"minecraft:bamboo\",count:1}},drop_chances:{mainhand:2.0f}}",
        );
        let id = spawn_saved(&mut host, &tag).unwrap();
        let saved = saved_tag(&host, id, Some(tag));
        let seen = look(&saved);
        for key in ["equipment", "drop_chances", "ForcedAge", "Age", "InLove"] {
            assert_eq!(seen.get(key), None, "{key} changes while it lives");
        }
        assert_eq!(seen.get("MainGene"), Some(&Tag::String("lazy".to_owned())));
        assert_eq!(seen.get("Baby"), Some(&Tag::Byte(1)));
        let mut friend = EntityWorld::default();
        assert!(spawn_tracked(&mut friend, id, &saved));
        let mirror = panda(&friend, id);
        assert!(mirror.core.baby);
        assert_eq!(
            (mirror.state.main_gene, mirror.state.hidden_gene),
            (Gene::Lazy, Gene::Worried)
        );
        assert_eq!(crate::families::tracking_range(pandas::KIND), Some(10));
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!((show.flags, show.values[HELD_VALUE]), (0, HELD_BAMBOO));
        let mob = host.family_mob_mut::<Panda>(id).unwrap();
        mob.state.held = Some(ItemStack::new(CAKE, 1));
        let goals = mob.state.goals_mut();
        goals.sitting = true;
        goals.on_back = true;
        goals.rolling = true;
        goals.sneezing = true;
        goals.scared = true;
        goals.unhappy = 20;
        let show = shows(&host)[&id].family.unwrap();
        assert_eq!(
            (
                show.flags,
                show.values[UNHAPPY_VALUE],
                show.values[HELD_VALUE]
            ),
            (
                SITTING | ON_BACK | ROLLING | SNEEZING | SCARED,
                20,
                HELD_CAKE
            )
        );
        Panda::show_on(friend.family_mob_mut::<Panda>(id).unwrap(), &show);
        let mirror = &panda(&friend, id).state;
        let goals = mirror.goals().unwrap();
        assert!(goals.sitting && goals.on_back && goals.rolling && goals.sneezing && goals.scared);
        assert_eq!(goals.unhappy, 20);
        assert_eq!(mirror.held.as_ref().map(|s| s.id.as_str()), Some(CAKE));
        Panda::show_on(
            friend.family_mob_mut::<Panda>(id).unwrap(),
            &FamilyShow::default(),
        );
        let mirror = &panda(&friend, id).state;
        let goals = mirror.goals().unwrap();
        assert!(!(goals.sitting || goals.on_back || goals.rolling || goals.sneezing));
        assert_eq!((goals.unhappy, mirror.held.as_ref()), (0, None));
        let stick = Some(ItemStack::new("minecraft:stick", 1));
        assert_eq!(held_value(stick.as_ref()), HELD_OTHER);
        assert_eq!(held_of(HELD_OTHER), None, "friends draw no other item");
    }

    #[test]
    fn bamboo_from_the_hand_brings_a_panda_into_love() {
        let mut world = EntityWorld::default();
        let tag = parse("{id:\"minecraft:panda\",Pos:[1.5d,64.0d,2.5d]}");
        let id = spawn_saved(&mut world, &tag).unwrap();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:wheat", 2));
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        let outcome = interact(&mut world, MobHit::Family(52, id), &mut actor);
        assert!(!outcome.handled, "wheat");
        inventory.slots[0] = Some(ItemStack::new("minecraft:bamboo", 2));
        let mut actor = Actor {
            player: 1,
            inventory: &mut inventory,
            selected: 0,
            infinite: false,
            entity_loot: None,
            shearing_loot: None,
        };
        let outcome = interact(&mut world, MobHit::Family(52, id), &mut actor);
        assert!(outcome.handled && outcome.inventory_changed);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(1));
        assert_eq!(panda(&world, id).state.in_love, 600);
        assert_eq!(world.take_entity_events(), vec![(id, 18)]);
    }
}
