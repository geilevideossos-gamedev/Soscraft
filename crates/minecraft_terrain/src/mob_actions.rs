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
use minecraftoss_entities::world::{EntityWorld, LeashUse, MobHit, PlayerAttack};
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
    pub mount: Option<u64>,
    pub mount_inventory: Option<u64>,
}

impl MobOutcome {
    fn sound(&mut self, event: impl Into<String>, position: DVec3, volume: f32, category: &'static str) {
        self.sounds.push(MobSound { event: event.into(), position, volume, pitch: 1.0, category });
    }
}

/// The player acting on a mob.
pub struct Actor<'a> {
    pub player: u64,
    pub inventory: &'a mut Inventory,
    pub selected: usize,
    /// Creative mode: held items are not used up or worn.
    pub infinite: bool,
    pub entity_loot: Option<&'a mut EntityLootBook>,
    pub shearing_loot: Option<&'a mut ShearingLootBook>,
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
    let sneaking = world.player_controls(actor.player).is_some_and(|controls| controls.sneaking);
    let leashed = world.leash_interact(hit.id(), actor.player, held.as_deref(), sneaking, infinite);
    if leashed != LeashUse::Pass {
        outcome.handled = true;
        match leashed {
            LeashUse::Tied if !infinite => {
                minecraftoss_entities::animal::consume_one(actor.inventory, selected);
                outcome.inventory_changed = true;
            }
            LeashUse::Snipped if !infinite => {
                if actor.inventory.wear_tool(selected, 1) {
                    outcome.broken.push("minecraft:shears".to_owned());
                }
                outcome.inventory_changed = true;
            }
            _ => {}
        }
        outcome.drops.extend(world.take_lead_drops().into_iter().map(|at| (ItemStack::new(minecraftoss_entities::world::LEAD, 1), at)));
        return outcome;
    }
    match hit {
        MobHit::Cow(id) if world.cows().iter().any(|e| e.id == id && e.horse.is_some()) => {
            if world.vehicle_of(minecraftoss_entities::world::Rider::Player(actor.player)).is_none() && world.horse_opens_inventory(id, actor.player) {
                outcome.handled = true;
                outcome.mount_inventory = Some(id);
                return outcome;
            }
            if world.horse_rides_on_use(id, actor.player, held.as_deref()) {
                outcome.handled = true;
                outcome.mount = Some(id);
                return outcome;
            }
            if world.horse_interact(id, actor.player, actor.inventory, selected, infinite) == InteractionResult::Pass {
                return outcome;
            }
        }
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
        MobHit::Strider(id) => {
            let food = held.as_deref().is_some_and(|item| recipes.item_in_tag(minecraftoss_entities::strider::FOOD_TAG, item));
            let sneaking = world.player_controls(actor.player).is_some_and(|controls| controls.sneaking);
            let empty = world.passengers_of(id).is_empty();
            let Some(entity) = world.strider_mut(id) else { return outcome };
            if !food && entity.strider.saddled && entity.strider.health > 0.0 && empty && !sneaking {
                outcome.handled = true;
                outcome.mount = Some(id);
                return outcome;
            }
            let position = entity.position();
            let (result, events) = entity.strider.interact_food(actor.inventory, selected, infinite, |item| recipes.item_in_tag(minecraftoss_entities::strider::FOOD_TAG, item));
            if result != InteractionResult::Pass {
                let (a, b) = (entity.random.next_float(), entity.random.next_float());
                outcome.sounds.push(MobSound { event: "entity.strider.eat".to_owned(), position, volume: 1.0, pitch: 1.0 + (a - b) * 0.2, category: "neutral_volume" });
                age_lock_sound(&mut outcome, animal_lock(&events), position);
            } else if entity.strider.interact_saddle(actor.inventory, selected, infinite) != InteractionResult::Pass {
                outcome.sound("entity.strider.saddle", position, 1.0, "neutral_volume");
            } else {
                return outcome;
            }
        }
        MobHit::Hoglin(id) => {
            let Some(entity) = world.hoglin_mut(id) else { return outcome };
            let pacified = entity.brain.memories.pacified.present();
            let (result, events) = entity.hoglin.interact_food(actor.inventory, selected, infinite, |item| recipes.item_in_tag(minecraftoss_entities::hoglin::FOOD_TAG, item), pacified);
            let position = entity.position();
            if result == InteractionResult::Pass {
                return outcome;
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
        MobHit::SnowGolem(id) if held.as_deref() == Some("minecraft:shears") => {
            use minecraftoss_entities::snow_golem::{SHEAR_LOOT, SHEAR_SOUND};
            let Some((position, eye)) = world.shear_snow_golem(id) else { return outcome };
            outcome.sound(SHEAR_SOUND, position, 1.0, "players_volume");
            let at = position + DVec3::new(0.0, f64::from(eye), 0.0);
            let drops = actor.shearing_loot.as_deref_mut().and_then(|loot| loot.roll_items(SHEAR_LOOT)).unwrap_or_else(Vec::new);
            outcome.drops.extend(drops.into_iter().map(|stack| (stack, at)));
            if !infinite && actor.inventory.wear_tool(selected, 1) {
                outcome.broken.push("minecraft:shears".to_owned());
            }
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
        MobHit::WanderingTrader(id) => {
            if world.wandering_trader_interact(id, actor.player, held.as_deref()) == minecraftoss_entities::world::VillagerUse::Pass {
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
        MobHit::Feline(id) => {
            if world.feline_interact(id, actor.player, actor.inventory, selected, infinite) == InteractionResult::Pass {
                return outcome;
            }
        }
        MobHit::Piglin(id) => {
            let Some(drops) = world.piglin_admire_held(id, actor.inventory, selected, infinite) else { return outcome };
            outcome.drops.extend(drops);
        }
        MobHit::Zombie(id) if held.as_deref() == Some("minecraft:golden_apple") => {
            let Some(started) = world.start_curing(id, actor.player) else { return outcome };
            if started && !infinite {
                minecraftoss_entities::animal::consume_one(actor.inventory, selected);
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
    use minecraftoss_entities::hoglin::{Hoglin, HoglinKind};
    use minecraftoss_entities::iron_golem::IronGolem;
    use minecraftoss_entities::mooshroom::{MushroomCow, MushroomVariant};
    use minecraftoss_entities::pig::Pig;
    use minecraftoss_entities::piglin::{Piglin, PiglinKind};
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
        let book = tagged_book(&[("cow_food", &["minecraft:wheat"]), ("sheep_food", &["minecraft:wheat"]), ("pig_food", &["minecraft:carrot"]), ("chicken_food", &["minecraft:wheat_seeds"]), ("strider_food", &["minecraft:warped_fungus"]), ("hoglin_food", &["minecraft:crimson_fungus"])]);
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
        let offer = minecraftoss_entities::trading::MerchantOffer {
            buy: minecraftoss_entities::trading::ItemCost { id: "minecraft:wheat".to_owned(), count: 20, components: None },
            buy_b: None,
            sell: minecraftoss_entities::trading::TradeItem { id: "minecraft:emerald".to_owned(), count: 1, components: serde_json::Map::new() },
            uses: 0,
            max_uses: 16,
            reward_exp: true,
            special_price: 0,
            demand: 0,
            price_multiplier: 0.05,
            xp: 2,
        };
        let trader = world.spawn_villager(Villager::new(at), true);
        world.villager_mut(trader).unwrap().offers = Some(vec![offer.clone()]);
        let mut young = Villager::new(at);
        young.age = baby(young.age);
        let young_villager = world.spawn_villager(young, true);
        world.villager_mut(young_villager).unwrap().offers = Some(vec![offer.clone()]);
        let sleeper = world.spawn_villager(Villager::new(at), true);
        world.villager_mut(sleeper).unwrap().offers = Some(vec![offer.clone()]);
        world.villager_mut(sleeper).unwrap().sleeping = Some((0, 64, 0));
        let busy = world.spawn_villager(Villager::new(at), true);
        world.villager_mut(busy).unwrap().offers = Some(vec![offer]);
        world.villager_mut(busy).unwrap().trading_player = Some(9);
        let wolf = world.spawn_wolf(Wolf::new(at), true);
        let chicken = world.spawn_chicken(minecraftoss_entities::chicken::Chicken::new(at), true);
        let hoglin = world.spawn_hoglin(Hoglin::new(HoglinKind::Hoglin, at), true);
        let mut young = Hoglin::new(HoglinKind::Hoglin, at);
        young.set_baby(true);
        let hoglet = world.spawn_hoglin(young, true);
        let calm = world.spawn_hoglin(Hoglin::new(HoglinKind::Hoglin, at), true);
        world.hoglin_mut(calm).unwrap().brain.memories.pacified.set_for((), 200);
        let zoglin = world.spawn_hoglin(Hoglin::new(HoglinKind::Zoglin, at), true);
        let piglin = world.spawn_piglin(Piglin::new(PiglinKind::Piglin, at), true);
        let mut young = Piglin::new(PiglinKind::Piglin, at);
        young.set_baby(true);
        let piglin_baby = world.spawn_piglin(young, true);
        let brute = world.spawn_piglin(Piglin::new(PiglinKind::Brute, at), true);
        let admiring = world.spawn_piglin(Piglin::new(PiglinKind::Piglin, at), true);
        world.piglin_mut(admiring).unwrap().brain.admire_for(50);
        let sulking = world.spawn_piglin(Piglin::new(PiglinKind::Piglin, at), true);
        world.piglin_mut(sulking).unwrap().brain.memories.admiring_disabled.set_for((), 400);
        use minecraftoss_entities::feline::{Feline, FelineKind};
        world.set_player_uuid(0, 77);
        let stray = world.spawn_feline(Feline::new(FelineKind::Cat, at), true);
        let mut pet = Feline::new(FelineKind::Cat, at);
        pet.tame = true;
        pet.owner = Some(77);
        let pet = world.spawn_feline(pet, true);
        let mut other = Feline::new(FelineKind::Cat, at);
        other.tame = true;
        other.owner = Some(78);
        let other = world.spawn_feline(other, true);
        let ocelot = world.spawn_feline(Feline::new(FelineKind::Ocelot, at), true);
        use minecraftoss_entities::horse::{HorseKind, HorseState};
        let equine = |world: &mut EntityWorld, kind: HorseKind, tame: bool| {
            let mut horse = HorseState::new(kind);
            horse.tame = tame;
            let mut cow = Cow::new(at);
            cow.max_health = 20.0;
            cow.health = 20.0;
            world.spawn_horse(cow, horse, true)
        };
        let wild_llama = equine(&mut world, HorseKind::Llama, false);
        let llama = equine(&mut world, HorseKind::Llama, true);
        let mule = equine(&mut world, HorseKind::Mule, true);
        let horse = equine(&mut world, HorseKind::Horse, true);
        let cases: &[(MobHit, Option<&str>, bool)] = &[
            (MobHit::Cow(wild_llama), Some("minecraft:hay_block"), true),
            (MobHit::Cow(wild_llama), Some("minecraft:chest"), true),
            (MobHit::Cow(wild_llama), None, true),
            (MobHit::Cow(llama), Some("minecraft:chest"), true),
            (MobHit::Cow(llama), Some("minecraft:white_carpet"), true),
            (MobHit::Cow(llama), Some("minecraft:carrot"), true),
            (MobHit::Cow(mule), Some("minecraft:chest"), true),
            (MobHit::Cow(mule), Some("minecraft:white_carpet"), true),
            (MobHit::Cow(mule), Some("minecraft:bucket"), true),
            (MobHit::Cow(horse), Some("minecraft:golden_carrot"), true),
            (MobHit::Cow(horse), Some("minecraft:chest"), true),
            (MobHit::Cow(horse), Some("minecraft:saddle"), true),
            (MobHit::Cow(wild_llama), Some("minecraft:saddle"), true),
            (MobHit::Feline(stray), Some("minecraft:cod"), true),
            (MobHit::Feline(stray), Some("minecraft:stick"), false),
            (MobHit::Feline(pet), None, true),
            (MobHit::Feline(pet), Some("minecraft:red_dye"), true),
            (MobHit::Feline(pet), Some("minecraft:blue_dye"), true),
            (MobHit::Feline(other), None, false),
            (MobHit::Feline(ocelot), Some("minecraft:salmon"), true),
            (MobHit::Feline(ocelot), None, false),
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
            (MobHit::Villager(trader), Some("minecraft:emerald"), true),
            (MobHit::Villager(trader), None, true),
            (MobHit::Villager(trader), Some("minecraft:villager_spawn_egg"), false),
            (MobHit::Villager(villager), None, true),
            (MobHit::Villager(young_villager), None, true),
            (MobHit::Villager(sleeper), None, false),
            (MobHit::Villager(busy), Some("minecraft:emerald"), false),
            (MobHit::Hoglin(hoglin), Some("minecraft:crimson_fungus"), true),
            (MobHit::Hoglin(hoglin), Some("minecraft:warped_fungus"), false),
            (MobHit::Hoglin(hoglet), Some("minecraft:crimson_fungus"), true),
            (MobHit::Hoglin(calm), Some("minecraft:crimson_fungus"), false),
            (MobHit::Hoglin(zoglin), Some("minecraft:crimson_fungus"), false),
            (MobHit::Piglin(piglin), Some("minecraft:gold_ingot"), true),
            (MobHit::Piglin(piglin), Some("minecraft:gold_nugget"), false),
            (MobHit::Piglin(piglin), Some("minecraft:golden_carrot"), false),
            (MobHit::Piglin(piglin_baby), Some("minecraft:gold_ingot"), false),
            (MobHit::Piglin(brute), Some("minecraft:gold_ingot"), false),
            (MobHit::Piglin(admiring), Some("minecraft:gold_ingot"), false),
            (MobHit::Piglin(sulking), Some("minecraft:gold_ingot"), false),
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
        assert!(world.villagers().iter().all(|e| e.id == busy || e.trading_player.is_none()), "nor starts a trade");
        assert!(world.merchant_menu(0).is_none());
    }

    #[test]
    fn a_right_clicked_ingot_is_used_and_admired() {
        for infinite in [false, true] {
            let at = DVec3::new(0.5, 64.0, 0.5);
            let mut world = EntityWorld::default();
            let mut piglin = Piglin::new(PiglinKind::Piglin, at);
            piglin.offhand = Some(ItemStack::new("minecraft:golden_carrot", 1));
            let id = world.spawn_piglin(piglin, true);
            let mut inventory = holding(Some("minecraft:gold_ingot"));
            inventory.slots[0].as_mut().unwrap().count = 3;
            let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite, entity_loot: None, shearing_loot: None, player: 0 };
            let outcome = interact(&mut world, MobHit::Piglin(id), &mut actor);
            assert!(outcome.handled && outcome.inventory_changed);
            let left = inventory.slots[0].as_ref().unwrap().count;
            assert_eq!(left, if infinite { 3 } else { 2 });
            let entity = world.piglin_mut(id).unwrap();
            assert_eq!(entity.piglin.offhand.as_ref().map(|s| s.id.as_str()), Some("minecraft:gold_ingot"));
            assert_eq!(entity.brain.memories.admiring_item.ttl(), Some(119));
            assert_eq!(outcome.drops.len(), 1, "the carrot it held falls");
            assert_eq!(outcome.drops[0].0.id, "minecraft:golden_carrot");
            let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite, entity_loot: None, shearing_loot: None, player: 0 };
            assert!(!interact(&mut world, MobHit::Piglin(id), &mut actor).handled, "one at a time");
        }
    }

    #[test]
    fn a_brute_ignores_a_gold_ingot() {
        let mut world = EntityWorld::default();
        let brute = world.spawn_piglin(Piglin::new(PiglinKind::Brute, DVec3::new(0.5, 64.0, 0.5)), true);
        let mut inventory = holding(Some("minecraft:gold_ingot"));
        let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite: false, entity_loot: None, shearing_loot: None, player: 0 };
        assert!(!interact(&mut world, MobHit::Piglin(brute), &mut actor).handled);
        assert_eq!(inventory.slots[0].as_ref().unwrap().count, 4);
        assert!(world.piglin_mut(brute).unwrap().piglin.offhand.is_none());
    }

    struct Floor;
    impl minecraftoss_player::World for Floor {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.1 < 64).then(|| minecraftoss_player::Block::new("minecraft:netherrack"))
        }
        fn set_block(&mut self, _pos: minecraftoss_player::Pos, _block: Option<minecraftoss_player::Block>) {}
    }

    fn strider(world: &mut EntityWorld, x: f64, baby: bool, saddled: bool) -> u64 {
        let mut strider = minecraftoss_entities::strider::Strider::new(DVec3::new(x, 64.0, 0.5));
        if baby {
            strider.age = baby_age(strider.age);
        }
        strider.saddled = saddled;
        world.spawn_strider(strider, false)
    }

    fn baby_age(age: Age) -> Age {
        baby(age)
    }

    fn use_on(world: &mut EntityWorld, hit: MobHit, item: &str) -> (MobOutcome, Inventory) {
        let mut inventory = holding(Some(item));
        let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite: false, entity_loot: None, shearing_loot: None, player: 0 };
        let outcome = interact(world, hit, &mut actor);
        (outcome, inventory)
    }

    #[test]
    fn right_click_mounts_a_saddled_strider_unless_crouching_or_holding_fungus() {
        let mut world = EntityWorld::default();
        let saddled = strider(&mut world, 0.5, false, true);
        let bare = strider(&mut world, 4.5, false, false);
        let (outcome, inventory) = use_on(&mut world, MobHit::Strider(saddled), "minecraft:warped_fungus_on_a_stick");
        assert!(outcome.handled && outcome.sounds.is_empty());
        assert_eq!(outcome.mount, Some(saddled));
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4), "mounting uses nothing up");
        assert!(would_interact(&world, MobHit::Strider(saddled), &holding(None), 0, false, 0), "an empty hand mounts too");
        let (outcome, _) = use_on(&mut world, MobHit::Strider(saddled), "minecraft:warped_fungus");
        assert_eq!(outcome.mount, None, "fungus feeds instead");
        assert!(outcome.handled);
        let (outcome, _) = use_on(&mut world, MobHit::Strider(bare), "minecraft:warped_fungus_on_a_stick");
        assert!(!outcome.handled && outcome.mount.is_none(), "no saddle, no ride");
        world.set_player_controls(&[minecraftoss_entities::controls::PlayerControls { id: 0, sneaking: true, ..Default::default() }]);
        let (outcome, _) = use_on(&mut world, MobHit::Strider(saddled), "minecraft:warped_fungus_on_a_stick");
        assert!(!outcome.handled && outcome.mount.is_none(), "crouching never mounts");
        world.set_player_controls(&[minecraftoss_entities::controls::PlayerControls { id: 0, ..Default::default() }]);
        assert!(world.force_riding(minecraftoss_entities::world::Rider::Player(7), saddled));
        let (outcome, _) = use_on(&mut world, MobHit::Strider(saddled), "minecraft:warped_fungus_on_a_stick");
        assert!(outcome.mount.is_none(), "one rider at a time");
    }

    #[test]
    fn right_click_mounts_a_horse_and_a_saddle_goes_on_a_tame_one() {
        use minecraftoss_entities::horse::{HorseKind, HorseState};
        let mut world = EntityWorld::default();
        let mut horse = HorseState::new(HorseKind::Horse);
        horse.max_health = 20.0;
        let id = world.spawn_horse(Cow::new(DVec3::new(0.5, 64.0, 0.5)), horse, false);
        let mut inventory = holding(None);
        let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite: false, entity_loot: None, shearing_loot: None, player: 0 };
        let outcome = interact(&mut world, MobHit::Cow(id), &mut actor);
        assert!(outcome.handled && outcome.mount == Some(id), "an empty hand mounts a wild horse to tame it");
        let (outcome, _) = use_on(&mut world, MobHit::Cow(id), "minecraft:saddle");
        assert!(outcome.handled && outcome.mount.is_none(), "a wild horse rears at the saddle");
        assert!(!world.cow_mut(id).unwrap().horse.as_ref().unwrap().saddled);
        world.cow_mut(id).unwrap().horse.as_mut().unwrap().tame = true;
        let (outcome, inventory) = use_on(&mut world, MobHit::Cow(id), "minecraft:saddle");
        assert!(outcome.handled && outcome.mount.is_none());
        assert!(world.cow_mut(id).unwrap().horse.as_ref().unwrap().saddled);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
        let (outcome, _) = use_on(&mut world, MobHit::Cow(id), "minecraft:stick");
        assert_eq!(outcome.mount, Some(id), "a tame horse is ridden with any other item");
        world.set_player_controls(&[minecraftoss_entities::controls::PlayerControls { id: 0, sneaking: true, ..Default::default() }]);
        let (outcome, _) = use_on(&mut world, MobHit::Cow(id), "minecraft:stick");
        assert_eq!((outcome.mount, outcome.mount_inventory), (None, Some(id)), "crouching on a tame horse opens its inventory");
        world.set_player_controls(&[minecraftoss_entities::controls::PlayerControls { id: 0, ..Default::default() }]);
        assert!(world.force_riding(minecraftoss_entities::world::Rider::Player(7), id));
        let (outcome, _) = use_on(&mut world, MobHit::Cow(id), "minecraft:stick");
        assert!(outcome.mount.is_none(), "one rider at a time");
    }

    #[test]
    fn two_fed_striders_make_a_baby() {
        let mut world = EntityWorld::default();
        let first = strider(&mut world, 0.5, false, false);
        let second = strider(&mut world, 2.5, false, false);
        for id in [first, second] {
            let (outcome, inventory) = use_on(&mut world, MobHit::Strider(id), "minecraft:warped_fungus");
            assert!(outcome.handled);
            assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
            assert_eq!(outcome.sounds.iter().map(|s| s.event.as_str()).collect::<Vec<_>>(), ["entity.strider.eat"]);
            assert!(world.strider_mut(id).unwrap().strider.in_love > 0);
        }
        for _ in 0..200 {
            world.tick_with_players(&mut Floor, &[]);
        }
        let babies = world.striders().iter().filter(|e| e.strider.age.baby()).count();
        assert_eq!((world.striders().len(), babies), (3, 1));
        assert!(world.striders().iter().filter(|e| !e.strider.age.baby()).all(|e| e.strider.age.ticks > 0 && e.strider.in_love == 0));
    }

    fn hoglin(world: &mut EntityWorld, x: f64, baby: bool) -> u64 {
        let mut hoglin = Hoglin::new(HoglinKind::Hoglin, DVec3::new(x, 64.0, 0.5));
        hoglin.set_baby(baby);
        let id = world.spawn_hoglin(hoglin, false);
        world.hoglin_mut(id).unwrap().set_random_seed(id);
        id
    }

    #[test]
    fn two_fed_hoglins_make_a_persistent_piglet_within_110_ticks() {
        let mut world = EntityWorld::default();
        let first = hoglin(&mut world, 0.5, false);
        let second = hoglin(&mut world, 3.5, false);
        for id in [first, second] {
            let (outcome, inventory) = use_on(&mut world, MobHit::Hoglin(id), "minecraft:crimson_fungus");
            assert!(outcome.handled);
            assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
            let entity = world.hoglin_mut(id).unwrap();
            assert!(entity.hoglin.in_love > 0 && entity.hoglin.persistence_required);
        }
        let mut born = None;
        for tick in 0..140 {
            world.tick_with_players(&mut Floor, &[]);
            if born.is_none() && world.hoglins().len() == 3 {
                born = Some(tick);
            }
        }
        assert!(born.is_some_and(|tick| tick <= 130), "{born:?}");
        let piglet = world.hoglins().iter().find(|e| e.hoglin.is_baby()).unwrap();
        assert!(piglet.hoglin.persistence_required && piglet.hoglin.age.ticks < -23000);
        for id in [first, second] {
            let entity = world.hoglin_mut(id).unwrap();
            assert!(entity.hoglin.age.ticks > 5800 && entity.hoglin.in_love == 0);
            assert!(!entity.brain.memories.breed_target.present());
        }
        let orbs = world.take_trade_experience();
        assert_eq!(orbs.len(), 1);
        assert!((1..=7).contains(&orbs[0].1));
    }

    #[test]
    fn a_pacified_hoglin_will_not_eat() {
        let mut world = EntityWorld::default();
        let calm = hoglin(&mut world, 0.5, false);
        world.hoglin_mut(calm).unwrap().brain.memories.pacified.set_for((), 200);
        let (outcome, inventory) = use_on(&mut world, MobHit::Hoglin(calm), "minecraft:crimson_fungus");
        assert!(!outcome.handled);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4));
        assert_eq!(world.hoglin_mut(calm).unwrap().hoglin.in_love, 0);
    }

    #[test]
    fn feeding_a_piglet_ages_it_by_a_tenth() {
        let mut world = EntityWorld::default();
        let piglet = hoglin(&mut world, 0.5, true);
        let (outcome, _) = use_on(&mut world, MobHit::Hoglin(piglet), "minecraft:crimson_fungus");
        assert!(outcome.handled);
        let entity = world.hoglin_mut(piglet).unwrap();
        assert_eq!(entity.hoglin.age.ticks, -24000 + 2400);
        assert!(entity.hoglin.persistence_required);
    }

    #[test]
    fn a_saddle_goes_on_an_adult_but_not_on_a_baby() {
        let mut world = EntityWorld::default();
        let adult = strider(&mut world, 0.5, false, false);
        let young = strider(&mut world, 4.5, true, false);
        let (outcome, inventory) = use_on(&mut world, MobHit::Strider(adult), "minecraft:saddle");
        assert!(outcome.handled && world.strider_mut(adult).unwrap().strider.saddled);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
        assert_eq!(outcome.sounds.iter().map(|s| s.event.as_str()).collect::<Vec<_>>(), ["entity.strider.saddle"]);
        let (outcome, inventory) = use_on(&mut world, MobHit::Strider(adult), "minecraft:saddle");
        assert_eq!(outcome.mount, Some(adult), "a second saddle rides it instead");
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4));
        let (outcome, inventory) = use_on(&mut world, MobHit::Strider(young), "minecraft:saddle");
        assert!(!outcome.handled && !world.strider_mut(young).unwrap().strider.saddled);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4));
        assert!(would_interact(&world, MobHit::Strider(adult), &holding(Some("minecraft:warped_fungus")), 0, false, 0));
        assert!(!would_interact(&world, MobHit::Strider(young), &holding(Some("minecraft:saddle")), 0, false, 0));
        assert!(would_interact(&world, MobHit::Strider(adult), &holding(Some("minecraft:carrot")), 0, false, 0), "a saddled adult is ridden with any other item");
        assert!(!would_interact(&world, MobHit::Strider(young), &holding(Some("minecraft:carrot")), 0, false, 0));
    }

    #[test]
    fn warped_fungus_feeds_even_a_saddled_strider() {
        let mut world = EntityWorld::default();
        let saddled = strider(&mut world, 0.5, false, true);
        let (outcome, inventory) = use_on(&mut world, MobHit::Strider(saddled), "minecraft:warped_fungus");
        assert!(outcome.handled);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
        assert!(world.strider_mut(saddled).unwrap().strider.in_love > 0);
        let young = strider(&mut world, 4.5, true, false);
        let before = world.strider_mut(young).unwrap().strider.age.ticks;
        let (outcome, _) = use_on(&mut world, MobHit::Strider(young), "minecraft:warped_fungus");
        assert!(outcome.handled);
        assert!(world.strider_mut(young).unwrap().strider.age.ticks > before, "a baby grows");
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

    #[test]
    fn a_golden_apple_cures_only_a_weakened_zombie_villager_and_is_used_up_then() {
        use minecraftoss_entities::effects::{EffectInstance, MobEffect};
        use minecraftoss_entities::zombie::{Zombie, ZombieKind};
        let mut world = EntityWorld::default();
        let mut villager = Zombie::new(DVec3::new(0.5, 1.0, 2.0));
        villager.kind = ZombieKind::ZombieVillager;
        let villager = world.spawn_zombie(villager, true);
        let plain = world.spawn_zombie(Zombie::new(DVec3::new(4.5, 1.0, 2.0)), true);
        let apple = "minecraft:golden_apple";
        let (outcome, inventory) = use_on(&mut world, MobHit::Zombie(plain), apple);
        assert!(!outcome.handled && inventory.slots[0].as_ref().map(|s| s.count) == Some(4), "a plain zombie lets the player eat");
        assert!(!would_interact(&world, MobHit::Zombie(plain), &holding(Some(apple)), 0, false, 0));
        let (outcome, inventory) = use_on(&mut world, MobHit::Zombie(villager), apple);
        assert!(outcome.handled && inventory.slots[0].as_ref().map(|s| s.count) == Some(4), "without weakness the click is taken and the apple kept");
        assert!(!world.zombie_mut(villager).unwrap().zombie.curing());
        assert!(world.add_mob_effect(villager, EffectInstance::new(MobEffect::Weakness, 1800, 0)));
        assert!(would_interact(&world, MobHit::Zombie(villager), &holding(Some(apple)), 0, false, 0));
        assert!(!world.zombie_mut(villager).unwrap().zombie.curing(), "asking changes nothing");
        let (outcome, inventory) = use_on(&mut world, MobHit::Zombie(villager), apple);
        assert!(outcome.handled && outcome.inventory_changed);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3), "one apple is used");
        assert!(world.zombie_mut(villager).unwrap().zombie.curing());
        let (_, inventory) = use_on(&mut world, MobHit::Zombie(villager), "minecraft:apple");
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4), "other food does nothing");
    }

    #[test]
    fn shears_take_a_snow_golem_s_pumpkin_once_and_drop_it_at_its_eyes() {
        use minecraftoss_entities::snow_golem::{SnowGolem, EYE_HEIGHT};
        let mut world = EntityWorld::default();
        let golem = world.spawn_snow_golem(SnowGolem::new(DVec3::new(2.5, 64.0, 0.5)), true);
        let inventory = holding(Some("minecraft:shears"));
        assert!(would_interact(&world, MobHit::SnowGolem(golem), &inventory, 0, false, 0));
        assert!(!would_interact(&world, MobHit::SnowGolem(golem), &holding(Some("minecraft:stick")), 0, false, 0));
        let mut loot = std::env::var_os("MINECRAFTOSS_ROOT").and_then(|root| ShearingLootBook::from_jar(&std::path::Path::new(&root).join("client.jar"), 0).ok());
        let mut inventory = holding(Some("minecraft:shears"));
        let mut actor = Actor { inventory: &mut inventory, selected: 0, infinite: false, entity_loot: None, shearing_loot: loot.as_mut(), player: 0 };
        let outcome = interact(&mut world, MobHit::SnowGolem(golem), &mut actor);
        assert!(outcome.handled);
        assert!(outcome.sounds.iter().any(|s| s.event == "entity.snow_golem.shear" && s.category == "players_volume"));
        assert!(!world.snow_golems()[0].golem.pumpkin);
        if loot.is_some() {
            assert_eq!(outcome.drops.len(), 1);
            assert_eq!(outcome.drops[0].0.id, "minecraft:carved_pumpkin");
            assert_eq!(outcome.drops[0].1, DVec3::new(2.5, 64.0 + f64::from(EYE_HEIGHT), 0.5));
        }
        let (again, _) = use_on(&mut world, MobHit::SnowGolem(golem), "minecraft:shears");
        assert!(!again.handled, "a bare snow golem has nothing to shear");
    }

    #[test]
    fn a_lead_ties_a_sheep_to_the_player_and_any_click_lets_it_go_with_its_lead() {
        use minecraftoss_entities::world::LeashHolder;
        let mut world = EntityWorld::default();
        let sheep = world.spawn_sheep_no_ai(Sheep::default(), DVec3::new(2.5, 64.0, 0.5));
        let zombie = world.spawn_zombie(minecraftoss_entities::zombie::Zombie::new(DVec3::new(1.5, 64.0, 0.5)), true);
        let player = minecraftoss_entities::tempt::PlayerCandidate {
            id: 0,
            position: DVec3::new(0.5, 64.0, 0.5),
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
        };
        world.set_leash_players(&[player]);
        let inventory = holding(Some("minecraft:lead"));
        assert!(would_interact(&world, MobHit::Sheep(sheep), &inventory, 0, false, 0));
        assert!(!would_interact(&world, MobHit::Zombie(zombie), &inventory, 0, false, 0), "monsters take no lead");
        assert_eq!(world.leash_holder(sheep), None, "would_interact leaves the world alone");
        let (outcome, inventory) = use_on(&mut world, MobHit::Sheep(sheep), "minecraft:lead");
        assert!(outcome.handled && outcome.inventory_changed);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3), "one lead used");
        assert_eq!(world.leash_holder(sheep), Some(LeashHolder::Player(0)));
        let (outcome, _) = use_on(&mut world, MobHit::Sheep(sheep), "minecraft:shears");
        assert!(outcome.handled);
        assert_eq!(world.leash_holder(sheep), None, "shears cut the lead before the wool");
        assert_eq!(outcome.drops.iter().map(|(stack, _)| stack.id.as_str()).collect::<Vec<_>>(), ["minecraft:lead"]);
        use_on(&mut world, MobHit::Sheep(sheep), "minecraft:lead");
        let (outcome, inventory) = use_on(&mut world, MobHit::Sheep(sheep), "minecraft:wheat");
        assert!(outcome.handled && outcome.drops.len() == 1);
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(4), "untying feeds nothing");
        assert_eq!(world.leash_holder(sheep), None);
    }
}
