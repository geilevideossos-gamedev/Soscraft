//! A player's hit on a mob and use of a held item on one, as the server
//! runs them (`Player.attack`'s basic hit, `Mob.interact` and the species'
//! `mobInteract`). The caller presents the outcome: sounds, drops and
//! inventory changes. Both the authored scene's local entity world and the
//! integrated server's run through here.

use glam::DVec3;
use minecraftoss_entities::animal::AnimalEvent;
use minecraftoss_entities::cow::{CowEvent, InteractionResult};
use minecraftoss_entities::loot::{EntityLootBook, ShearingLootBook};
use minecraftoss_entities::mooshroom::flower_effects;
use minecraftoss_entities::sheep::ShearsResult;
use minecraftoss_entities::world::{EntityWorld, MobHit, PlayerAttack};
use minecraftoss_player::inventory::{Inventory, ItemStack};

/// A sound a mob action plays, with its sound category option.
#[derive(Clone, Debug, PartialEq)]
pub struct MobSound {
    pub event: String,
    pub position: DVec3,
    pub volume: f32,
    pub pitch: f32,
    pub category: &'static str,
}

/// What a hit or use did.
#[derive(Clone, Debug, Default)]
pub struct MobOutcome {
    /// The action was taken (the click goes no further).
    pub handled: bool,
    pub sounds: Vec<MobSound>,
    /// Items the mob dropped, where they appear.
    pub drops: Vec<(ItemStack, DVec3)>,
    /// Experience killed mobs left, where they died.
    pub experience: Vec<(DVec3, i32)>,
    /// The acting player's inventory changed.
    pub inventory_changed: bool,
    pub broken: Vec<String>,
}

impl MobOutcome {
    fn sound(&mut self, event: impl Into<String>, position: DVec3, volume: f32, category: &'static str) {
        self.sounds.push(MobSound { event: event.into(), position, volume, pitch: 1.0, category });
    }
}

/// The player acting on a mob.
pub struct Actor<'a> {
    pub inventory: &'a mut Inventory,
    pub selected: usize,
    /// Creative mode: held items are not used up or worn.
    pub infinite: bool,
    pub entity_loot: Option<&'a mut EntityLootBook>,
    pub shearing_loot: Option<&'a mut ShearingLootBook>,
    pub player: u64,
}


/// What the mobs that died since the last call leave where each died: their
/// drops ([`EntityLootBook::death_drops`]; without loot tables nothing
/// drops) and their experience.
pub fn death_remains(world: &mut EntityWorld, loot: Option<&mut EntityLootBook>) -> (Vec<(ItemStack, DVec3)>, Vec<(DVec3, i32)>) {
    let deaths = world.take_deaths();
    let experience = deaths.iter().filter(|d| d.experience > 0).map(|d| (d.position, d.experience)).collect();
    (loot.map(|book| book.death_drops(deaths)).unwrap_or_default(), experience)
}

/// Whether a hit can land on the mob `id` (a creeper that blew up is gone).
fn hittable(world: &EntityWorld, id: u64) -> bool {
    world.mob_ids().any(|mob| mob == id) && !world.creepers().iter().any(|e| e.id == id && e.creeper.exploded)
}

/// A player's hit on a mob (`Player.attack`): the entity world applies it
/// ([`EntityWorld::player_attack`]); this presents it, with the player's
/// attack sounds, each hurt mob's sound and the loot of those it killed.
pub fn attack(world: &mut EntityWorld, hit: MobHit, actor: &mut Actor, attack: &PlayerAttack) -> MobOutcome {
    let id = hit.id();
    if !hittable(world, id) {
        return MobOutcome::default();
    }
    let result = world.player_attack(attack, id);
    let mut outcome = MobOutcome { handled: true, ..MobOutcome::default() };
    let at = attack.position;
    if result.knockback {
        outcome.sound("entity.player.attack.knockback", at, 1.0, "players_volume");
    }
    if !result.hurt {
        outcome.sound("entity.player.attack.nodamage", at, 1.0, "players_volume");
        return outcome;
    }
    (outcome.drops, outcome.experience) = death_remains(world, actor.entity_loot.as_deref_mut());
    if result.sweep {
        outcome.sound("entity.player.attack.sweep", at, 1.0, "players_volume");
    }
    if result.critical {
        outcome.sound("entity.player.attack.crit", at, 1.0, "players_volume");
    } else if !result.sweep {
        let event = if result.full_strength { "entity.player.attack.strong" } else { "entity.player.attack.weak" };
        outcome.sound(event, at, 1.0, "players_volume");
    }
    outcome.sounds.extend(world.take_sounds().into_iter().map(|s| MobSound { event: s.event, position: s.position, volume: s.volume, pitch: s.pitch, category: s.category }));
    outcome
}

/// The golden dandelion's age lock sound.
fn age_lock_sound(outcome: &mut MobOutcome, locked: Option<bool>, position: DVec3) {
    if let Some(locked) = locked {
        let event = if locked { "item.golden_dandelion.use" } else { "item.golden_dandelion.unuse" };
        outcome.sound(event, position, 1.0, "players_volume");
    }
}

fn animal_lock(events: &[AnimalEvent]) -> Option<bool> {
    events.iter().find_map(|event| match event {
        AnimalEvent::AgeLock => Some(true),
        AnimalEvent::AgeUnlock => Some(false),
        _ => None,
    })
}

fn cow_lock(events: &[CowEvent]) -> Option<bool> {
    events.iter().find_map(|event| match event {
        CowEvent::AgeLock => Some(true),
        CowEvent::AgeUnlock => Some(false),
        _ => None,
    })
}

/// Using the held item on a mob. A pass (nothing happens) leaves the
/// outcome unhandled, so the click may use the item or a block instead.
pub fn interact(world: &mut EntityWorld, hit: MobHit, actor: &mut Actor) -> MobOutcome {
    let selected = actor.selected;
    let infinite = actor.infinite;
    let recipes = actor.inventory.recipes.clone();
    let held = actor.inventory.slots.get(selected).and_then(Option::as_ref).map(|stack| stack.id.clone());
    let mut outcome = MobOutcome::default();
    match hit {
        MobHit::Cow(id) => {
            let Some(entity) = world.cow_mut(id) else { return outcome };
            let position = entity.cow.body.position;
            let (result, events) = entity.cow.interact(actor.inventory, selected, infinite, |item| recipes.item_in_tag("minecraft:cow_food", item), false);
            if result == InteractionResult::Pass {
                return outcome;
            }
            if events.contains(&CowEvent::Milk) {
                outcome.sound("entity.cow.milk", position, 1.0, "friendly_volume");
            }
        }
        MobHit::Sheep(id) if held.as_deref() == Some("minecraft:shears") => {
            let Some(loot) = actor.shearing_loot.as_deref_mut() else { return outcome };
            let Some(entity) = world.sheep_mut(id) else { return outcome };
            let position = entity.body.position;
            let Some(result) = entity.sheep.use_shears(loot) else { return outcome };
            if let ShearsResult::Sheared { drops, tool_damage } = result {
                outcome.sound("entity.sheep.shear", position, 1.0, "friendly_volume");
                for stack in drops {
                    for _ in 0..stack.count {
                        outcome.drops.push((ItemStack { count: 1, ..stack.clone() }, position + DVec3::Y));
                    }
                }
                if !infinite && actor.inventory.wear_tool(selected, tool_damage) {
                    outcome.broken.push("minecraft:shears".to_owned());
                }
            }
        }
        MobHit::Sheep(id) => {
            let Some(entity) = world.sheep_mut(id) else { return outcome };
            let (result, events) = entity.sheep.interact_food(actor.inventory, selected, infinite, |item| recipes.item_in_tag("minecraft:sheep_food", item), false);
            entity.sync_dimensions();
            let position = entity.body.position;
            if result == InteractionResult::Pass {
                return outcome;
            }
            age_lock_sound(&mut outcome, animal_lock(&events), position);
        }
        MobHit::Pig(id) => {
            let Some(entity) = world.pig_mut(id) else { return outcome };
            let was_saddled = entity.pig.saddled;
            let (result, events) = if held.as_deref() == Some("minecraft:saddle") {
                (entity.pig.interact_saddle(actor.inventory, selected, infinite), Vec::new())
            } else {
                entity.pig.interact_food(actor.inventory, selected, infinite, |item| recipes.item_in_tag("minecraft:pig_food", item), false)
            };
            entity.pig.sync_dimensions();
            let position = entity.pig.body.position;
            if result == InteractionResult::Pass {
                return outcome;
            }
            if !was_saddled && entity.pig.saddled {
                outcome.sound("entity.pig.saddle", position, 1.0, "friendly_volume");
            }
            age_lock_sound(&mut outcome, animal_lock(&events), position);
        }
        MobHit::Chicken(id) => {
            let Some(entity) = world.chicken_mut(id) else { return outcome };
            let (result, events) = entity.chicken.interact_food(actor.inventory, selected, infinite, |item| recipes.item_in_tag("minecraft:chicken_food", item), false);
            entity.chicken.sync_dimensions();
            let position = entity.chicken.body.position;
            if result == InteractionResult::Pass {
                return outcome;
            }
            age_lock_sound(&mut outcome, animal_lock(&events), position);
        }
        MobHit::Mooshroom(id) if held.as_deref() == Some("minecraft:shears") => {
            let Some(position) = world.mooshroom_mut(id).map(|entity| entity.cow.body.position) else { return outcome };
            let Some(shearing) = world.shear_mooshroom(id) else { return outcome };
            outcome.sound("entity.mooshroom.shear", position, 1.0, "players_volume");
            for _ in 0..shearing.drop_count {
                outcome.drops.push((ItemStack::new(shearing.drop_item, 1), position + DVec3::Y));
            }
            if !infinite && actor.inventory.wear_tool(selected, shearing.tool_damage) {
                outcome.broken.push("minecraft:shears".to_owned());
            }
        }
        MobHit::Mooshroom(id) => {
            let Some(entity) = world.mooshroom_mut(id) else { return outcome };
            let Some(state) = entity.mooshroom.as_mut() else { return outcome };
            let was_charged = state.stew_effects.is_some();
            let (result, events) = state.interact(&mut entity.cow, actor.inventory, selected, infinite, |item| recipes.item_in_tag("minecraft:cow_food", item), flower_effects);
            let charged = !was_charged && state.stew_effects.is_some();
            entity.cow.sync_dimensions();
            let position = entity.cow.body.position;
            if result == InteractionResult::Pass {
                return outcome;
            }
            if held.as_deref() == Some("minecraft:bowl") {
                let event = if was_charged { "entity.mooshroom.suspicious_milk" } else { "entity.mooshroom.milk" };
                outcome.sound(event, position, 1.0, "friendly_volume");
            } else if charged {
                outcome.sound("entity.mooshroom.eat", position, 2.0, "friendly_volume");
            }
            age_lock_sound(&mut outcome, cow_lock(&events), position);
        }
        MobHit::Villager(id) => {
            // `Villager.mobInteract`: trading opens, or it shakes its head.
            if world.villager_interact(id, actor.player, true, held.as_deref()) == minecraftoss_entities::world::VillagerUse::Pass {
                return outcome;
            }
            outcome.handled = true;
            return outcome;
        }
        MobHit::Wolf(id) => {
            // `Wolf.mobInteract`: taming with bones, feeding, collar dyes,
            // love, and its owner's orders to sit or stand.
            if world.wolf_interact(id, actor.player, actor.inventory, selected, infinite) == InteractionResult::Pass {
                return outcome;
            }
        }
        MobHit::IronGolem(id) if held.as_deref() == Some("minecraft:iron_ingot") => {
            let Some(entity) = world.iron_golem_mut(id) else { return outcome };
            if !entity.repair_with_ingot() {
                return outcome;
            }
            if !infinite {
                minecraftoss_entities::animal::consume_one(actor.inventory, selected);
            }
        }
        _ => return outcome,
    }
    outcome.handled = true;
    outcome.inventory_changed = true;
    outcome
}

pub fn would_interact(world: &EntityWorld, hit: MobHit, inventory: &Inventory, selected: usize, infinite: bool, player: u64) -> bool {
    let held = inventory.slots.get(selected).and_then(Option::as_ref).map(|stack| stack.id.as_str());
    match hit {
        MobHit::Villager(_) => false,
        MobHit::Sheep(id) if held == Some("minecraft:shears") => world.sheep().iter().any(|e| e.id == id),
        _ => {
            let mut world = world.clone();
            let mut inventory = inventory.clone();
            let mut actor = Actor { inventory: &mut inventory, selected, infinite, entity_loot: None, shearing_loot: None, player };
            interact(&mut world, hit, &mut actor).handled
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::age::Age;
    use minecraftoss_entities::cow::Cow;
    use minecraftoss_entities::iron_golem::IronGolem;
    use minecraftoss_entities::mooshroom::{MushroomCow, MushroomVariant};
    use minecraftoss_entities::pig::Pig;
    use minecraftoss_entities::sheep::Sheep;
    use minecraftoss_entities::villager::Villager;
    use minecraftoss_entities::wolf::Wolf;
    use minecraftoss_player::crafting::RecipeBook;
    use std::io::Write;

    pub(crate) fn tagged_book(tags: &[(&str, &[&str])]) -> RecipeBook {
        let dir = crate::test_dir::tempdir().unwrap();
        let jar = dir.path().join("client.jar");
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        for (tag, values) in tags {
            zip.start_file(format!("data/minecraft/tags/item/{tag}.json"), zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(serde_json::json!({ "values": values }).to_string().as_bytes()).unwrap();
        }
        zip.finish().unwrap();
        RecipeBook::from_jar(&jar).unwrap()
    }

    fn holding(item: Option<&str>) -> Inventory {
        let book = tagged_book(&[("cow_food", &["minecraft:wheat"]), ("sheep_food", &["minecraft:wheat"]), ("pig_food", &["minecraft:carrot"]), ("chicken_food", &["minecraft:wheat_seeds"])]);
        let mut inventory = Inventory::default().with_recipes(book);
        inventory.slots[0] = item.map(|id| ItemStack::new(id, 4));
        inventory
    }

    fn baby(mut age: Age) -> Age {
        age.ticks = Age::BABY_START;
        age
    }

    #[test]
    fn would_interact_agrees_with_interact() {
        let at = DVec3::new(0.5, 64.0, 0.5);
        let mut world = EntityWorld::default();
        let cow = world.spawn_cow(Cow::new(at), true);
        let mut young = Cow::new(at);
        young.age = baby(young.age);
        let calf = world.spawn_cow(young, true);
        let pig = world.spawn_pig_no_ai(Pig::new(at));
        let mut saddled = Pig::new(at);
        saddled.saddled = true;
        let saddled = world.spawn_pig_no_ai(saddled);
        let mut piglet = Pig::new(at);
        piglet.age = baby(piglet.age);
        let piglet = world.spawn_pig_no_ai(piglet);
        let sheep = world.spawn_sheep_no_ai(Sheep::default(), at);
        let mooshroom = world.spawn_mooshroom(MushroomCow::new(at, MushroomVariant::Red), true);
        let full_golem = world.spawn_iron_golem(IronGolem::new(at), true);
        let mut hurt = IronGolem::new(at);
        hurt.health = 50.0;
        let hurt_golem = world.spawn_iron_golem(hurt, true);
        let villager = world.spawn_villager(Villager::new(at), true);
        let wolf = world.spawn_wolf(Wolf::new(at), true);
        let chicken = world.spawn_chicken(minecraftoss_entities::chicken::Chicken::new(at), true);
        let cases: &[(MobHit, Option<&str>, bool)] = &[
            (MobHit::Cow(cow), Some("minecraft:wheat"), true),
            (MobHit::Cow(cow), Some("minecraft:bucket"), true),
            (MobHit::Cow(cow), Some("minecraft:carrot"), false),
            (MobHit::Cow(cow), None, false),
            (MobHit::Cow(calf), Some("minecraft:bucket"), false),
            (MobHit::Cow(calf), Some("minecraft:wheat"), true),
            (MobHit::Pig(pig), Some("minecraft:saddle"), true),
            (MobHit::Pig(pig), Some("minecraft:carrot"), true),
            (MobHit::Pig(pig), Some("minecraft:wheat"), false),
            (MobHit::Pig(saddled), Some("minecraft:saddle"), false),
            (MobHit::Pig(piglet), Some("minecraft:saddle"), false),
            (MobHit::Sheep(sheep), Some("minecraft:wheat"), true),
            (MobHit::Sheep(sheep), Some("minecraft:carrot"), false),
            (MobHit::Mooshroom(mooshroom), Some("minecraft:bowl"), true),
            (MobHit::Mooshroom(mooshroom), Some("minecraft:shears"), true),
            (MobHit::Mooshroom(mooshroom), Some("minecraft:stick"), false),
            (MobHit::IronGolem(hurt_golem), Some("minecraft:iron_ingot"), true),
            (MobHit::IronGolem(full_golem), Some("minecraft:iron_ingot"), false),
            (MobHit::IronGolem(hurt_golem), Some("minecraft:gold_ingot"), false),
            (MobHit::Chicken(chicken), Some("minecraft:wheat_seeds"), true),
            (MobHit::Chicken(chicken), Some("minecraft:wheat"), false),
            (MobHit::Wolf(wolf), Some("minecraft:bone"), true),
            (MobHit::Wolf(wolf), Some("minecraft:stick"), false),
            (MobHit::Villager(villager), Some("minecraft:emerald"), false),
            (MobHit::Villager(villager), None, false),
        ];
        for &(hit, held, expected) in cases {
            for infinite in [false, true] {
                let inventory = holding(held);
                let would = would_interact(&world, hit, &inventory, 0, infinite, 0);
                let mut copy = world.clone();
                let mut used = inventory.clone();
                let mut actor = Actor { inventory: &mut used, selected: 0, infinite, entity_loot: None, shearing_loot: None, player: 0 };
                let handled = interact(&mut copy, hit, &mut actor).handled;
                assert_eq!(would, handled, "{hit:?} with {held:?}");
                assert_eq!(would, expected, "{hit:?} with {held:?}");
            }
        }
        assert!(would_interact(&world, MobHit::Sheep(sheep), &holding(Some("minecraft:shears")), 0, false, 0), "shears always take on a sheep");
        assert!(!would_interact(&world, MobHit::Sheep(sheep + 1000), &holding(Some("minecraft:shears")), 0, false, 0));
        assert!(world.pig_mut(pig).is_some_and(|e| !e.pig.saddled), "the predicate leaves the world alone");
    }

    #[test]
    fn a_saddle_on_a_pig_is_used_up_with_its_sound() {
        let mut world = EntityWorld::default();
        let pig = world.spawn_pig_no_ai(Pig::new(DVec3::new(0.5, 64.0, 0.5)));
        let mut inventory = holding(Some("minecraft:saddle"));
        inventory.slots[0].as_mut().unwrap().count = 1;
        let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite: false, entity_loot: None, shearing_loot: None, player: 0 };
        let outcome = interact(&mut world, MobHit::Pig(pig), &mut actor);
        assert!(outcome.handled && outcome.inventory_changed);
        assert!(inventory.slots[0].is_none());
        assert!(world.pig_mut(pig).unwrap().pig.saddled);
        assert_eq!(outcome.sounds.iter().map(|s| s.event.as_str()).collect::<Vec<_>>(), ["entity.pig.saddle"]);
    }
}
