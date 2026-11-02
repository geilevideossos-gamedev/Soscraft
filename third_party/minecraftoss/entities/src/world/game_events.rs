use super::*;

#[derive(Clone, Debug, PartialEq)]
pub struct MobGameEvent {
    pub event: &'static str,
    pub pos: DVec3,
    pub mob: Option<u64>,
    pub owner: Option<u64>,
    pub affected: Option<String>,
}

impl EntityWorld {
    pub fn game_event(
        &mut self,
        event: &'static str,
        pos: DVec3,
        mob: Option<u64>,
        affected: Option<String>,
    ) {
        self.game_events.push(MobGameEvent {
            event,
            pos,
            mob,
            owner: None,
            affected,
        });
    }

    pub fn take_game_events(&mut self) -> Vec<MobGameEvent> {
        let ids: Vec<u64> = self.mob_ids().collect();
        for id in ids {
            let damaged = self
                .damage_state_mut(id)
                .is_some_and(|damage| std::mem::take(&mut damage.damaged));
            let Some(body) = self.body_mut(id) else {
                continue;
            };
            let moved: Vec<_> = body.game_events.drain(..).collect();
            let at = body.position;
            if damaged {
                self.game_event("minecraft:entity_damage", at, Some(id), None);
            }
            for (event, pos, affected) in moved {
                self.game_event(event, pos, Some(id), affected);
            }
        }
        std::mem::take(&mut self.game_events)
    }

    pub(super) fn rider_id(rider: Rider) -> u64 {
        match rider {
            Rider::Mob(id) => id,
            Rider::Player(player) => PLAYER_TARGET + player,
        }
    }

    pub(super) fn arrow_landed(&mut self, id: u64, owner: u64, at: DVec3) {
        let Some(arrow) = self
            .arrows
            .iter()
            .find(|entity| entity.id == id)
            .map(|entity| &entity.arrow)
        else {
            return;
        };
        let (pos, affected) = match arrow.impact_block.filter(|_| arrow.in_ground) {
            Some((x, y, z)) => (
                DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5),
                arrow.impact_state().map(|block| block.id.clone()),
            ),
            None => (at, None),
        };
        self.game_events.push(MobGameEvent {
            event: "minecraft:projectile_land",
            pos,
            mob: None,
            owner: Some(owner),
            affected,
        });
    }

    pub fn grounded(&mut self) -> Vec<(u64, DVec3)> {
        let ids: Vec<u64> = self.mob_ids().collect();
        ids.into_iter()
            .filter_map(|id| {
                let body = self.body_mut(id)?;
                body.on_ground.then_some((id, body.position))
            })
            .collect()
    }
}

impl Body {
    pub fn equipment_changed(&mut self, equippable: bool) {
        let event = if equippable {
            "minecraft:equip"
        } else {
            "minecraft:unequip"
        };
        self.game_events.push((event, self.position, None));
    }
}

pub fn equippable(recipes: Option<&RecipeBook>, stack: Option<&ItemStack>) -> bool {
    let Some(stack) = stack else {
        return false;
    };
    match recipes.filter(|recipes| recipes.item_catalog().is_some()) {
        Some(recipes) => recipes.equipment_slot(stack).is_some(),
        None => crate::piglin::armor_slot(&stack.id).is_some(),
    }
}

fn same_item(old: Option<&ItemStack>, new: Option<&ItemStack>) -> bool {
    match (old, new) {
        (Some(old), Some(new)) => old.id == new.id && old.components == new.components,
        (old, new) => old.is_none() && new.is_none(),
    }
}

impl crate::piglin::Piglin {
    pub fn equipment_changed(
        &mut self,
        recipes: Option<&RecipeBook>,
        slot: usize,
        old: Option<&ItemStack>,
    ) {
        let new = self.equipment(slot);
        if same_item(old, new) {
            return;
        }
        let equippable = equippable(recipes, new);
        self.body.equipment_changed(equippable);
    }
}

impl VillagerEntity {
    pub fn held_item_changed(
        &mut self,
        recipes: Option<&RecipeBook>,
        old: Option<&crate::trading::TradeItem>,
    ) {
        let stack = |item: &crate::trading::TradeItem| ItemStack {
            components: (!item.components.is_empty())
                .then(|| serde_json::Value::Object(item.components.clone())),
            ..ItemStack::new(item.id.clone(), 1)
        };
        let (old, new) = (old.map(stack), self.held_item.as_ref().map(stack));
        if same_item(old.as_ref(), new.as_ref()) {
            return;
        }
        let equippable = equippable(recipes, new.as_ref());
        self.villager.body.equipment_changed(equippable);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::piglin::{Piglin, PiglinKind};
    use crate::villager_brain::Tracker;
    use minecraftoss_player::{Pos, WorldItem};

    const DOOR: Pos = (0, 64, 3);

    #[derive(Default)]
    struct Yard {
        blocks: HashMap<Pos, Option<Block>>,
        items: Vec<WorldItem>,
    }

    impl Yard {
        fn door_wall() -> Self {
            let mut yard = Yard::default();
            for x in -8..=8 {
                for y in 64..=66 {
                    yard.blocks
                        .insert((x, y, 3), Some(Block::new("minecraft:stone")));
                }
            }
            for (y, half) in [(64, "lower"), (65, "upper")] {
                let door = Block::new("minecraft:oak_door")
                    .with("facing", "north")
                    .with("half", half)
                    .with("hinge", "left")
                    .with("open", "false")
                    .with("powered", "false");
                yard.blocks.insert((0, y, 3), Some(door));
            }
            yard
        }

        fn drop(&mut self, item: &str, at: DVec3) {
            self.items.push(WorldItem {
                id: self.items.len() as i32 + 1,
                position: at,
                item: item.to_owned(),
                count: 1,
                components: None,
                pickup_delay: 0,
            });
        }
    }

    impl World for Yard {
        fn block(&self, pos: Pos) -> Option<Block> {
            match self.blocks.get(&pos) {
                Some(block) => block.clone(),
                None => (pos.1 < 64).then(|| Block::new("minecraft:dirt")),
            }
        }

        fn set_block(&mut self, pos: Pos, block: Option<Block>) {
            self.blocks.insert(pos, block);
        }

        fn block_in_tag(&self, pos: Pos, tag: &str) -> bool {
            let Some(block) = self.block(pos) else {
                return false;
            };
            if tag == "minecraft:mob_interactable_doors" {
                return block.id == "minecraft:oak_door";
            }
            minecraftoss_player::authored_block_in_tag(&block, tag, &self.collision_boxes(pos))
        }

        fn items_in(&self, min: DVec3, max: DVec3) -> Vec<WorldItem> {
            self.items
                .iter()
                .filter(|item| item.position.cmpge(min).all() && item.position.cmple(max).all())
                .cloned()
                .collect()
        }

        fn item(&self, id: i32) -> Option<WorldItem> {
            self.items.iter().find(|item| item.id == id).cloned()
        }

        fn take_item(&mut self, id: i32, count: i32) {
            if let Some(item) = self.items.iter_mut().find(|item| item.id == id) {
                item.count -= count;
            }
            self.items.retain(|item| item.count > 0);
        }
    }

    fn posted(world: &mut EntityWorld, events: &[&str]) -> Vec<(&'static str, DVec3, Option<u64>)> {
        world
            .take_game_events()
            .into_iter()
            .filter(|event| events.contains(&event.event))
            .map(|event| (event.event, event.pos, event.mob))
            .collect()
    }

    fn centre(pos: Pos) -> DVec3 {
        DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2)) + 0.5
    }

    const DOORS: [&str; 2] = ["minecraft:block_open", "minecraft:block_close"];
    const EQUIPMENT: [&str; 2] = ["minecraft:equip", "minecraft:unequip"];

    #[test]
    fn a_silverfish_breaking_out_of_infested_stone_posts_block_destroy_and_hiding_in_it_nothing() {
        for griefing in [true, false] {
            let mut world = EntityWorld::default();
            world.set_mob_griefing(griefing);
            let mut yard = Yard::default();
            let infested: Vec<Pos> = (-2..=2).map(|x| (x, 64, 3)).collect();
            for &pos in &infested {
                yard.blocks
                    .insert(pos, Some(Block::new("minecraft:infested_stone")));
            }
            let fish = world
                .spawn_family_kind(
                    crate::world::families::pests::silverfish::KIND,
                    DVec3::new(0.5, 64.0, 0.5),
                )
                .unwrap();
            world.tick(&mut yard);
            world.take_game_events();
            let magic = crate::world::families::Hurt::new(
                1.0,
                "minecraft:magic",
                crate::world::DamageSourceKind::Magic,
            );
            assert!(world.hurt_family(fish, &magic).unwrap().applied);
            let mut broken = Vec::new();
            for _ in 0..40 {
                world.tick(&mut yard);
                broken.extend(posted(&mut world, &["minecraft:block_destroy"]));
            }
            let gone: Vec<Pos> = infested
                .iter()
                .copied()
                .filter(|pos| yard.block(*pos).is_none())
                .collect();
            if !griefing {
                assert!(broken.is_empty() && gone.is_empty(), "{broken:?}");
                continue;
            }
            assert!(!gone.is_empty());
            let expected: Vec<_> = gone
                .iter()
                .map(|&pos| ("minecraft:block_destroy", centre(pos), Some(fish)))
                .collect();
            broken.sort_by(|a, b| a.1.x.total_cmp(&b.1.x));
            assert_eq!(
                broken, expected,
                "from the silverfish at each block it broke"
            );
        }
        let mut world = EntityWorld::default();
        world.set_mob_griefing(true);
        let mut yard = Yard::default();
        for (x, z) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            for y in [64, 65] {
                yard.blocks
                    .insert((x, y, z), Some(Block::new("minecraft:stone_bricks")));
            }
        }
        let fish = world
            .spawn_family_kind(
                crate::world::families::pests::silverfish::KIND,
                DVec3::new(0.5, 64.0, 0.5),
            )
            .unwrap();
        let mut blocks = Vec::new();
        for _ in 0..2400 {
            world.tick(&mut yard);
            blocks.extend(
                world
                    .take_game_events()
                    .into_iter()
                    .filter(|event| event.event.starts_with("minecraft:block_")),
            );
            if world.entity_type(fish).is_none() {
                break;
            }
        }
        assert_eq!(world.entity_type(fish), None, "it hid in the wall");
        assert!(blocks.is_empty(), "hiding posts nothing: {blocks:?}");
    }

    fn door_events(
        world: &mut EntityWorld,
        yard: &mut Yard,
        ticks: usize,
        until: &str,
    ) -> Vec<(&'static str, DVec3, Option<u64>)> {
        let mut heard = Vec::new();
        for _ in 0..ticks {
            world.tick(yard);
            heard.extend(posted(world, &DOORS));
            if heard.iter().any(|(event, ..)| *event == until) {
                break;
            }
        }
        heard
    }

    fn villager_walking(world: &mut EntityWorld, from: DVec3, to: Pos) -> u64 {
        let id = world.spawn_villager_active(crate::villager::Villager::new(from), 0.0);
        let ai = world.villager_mut(id).unwrap().ai.as_mut().unwrap();
        ai.brain
            .memories
            .walk_target
            .set(crate::villager_brain::WalkTarget {
                target: Tracker::Block(to),
                speed: 0.5,
                close_enough: 0,
            });
        id
    }

    #[test]
    fn a_villager_and_a_piglin_opening_and_closing_a_door_post_block_open_and_block_close() {
        let mut yard = Yard::door_wall();
        let mut world = EntityWorld::default();
        let villager = villager_walking(&mut world, DVec3::new(0.5, 64.0, 0.5), (0, 64, 7));
        let heard = door_events(&mut world, &mut yard, 400, DOORS[0]);
        assert_eq!(heard, [(DOORS[0], centre(DOOR), Some(villager))]);
        let mut world = EntityWorld::default();
        let villager = villager_walking(&mut world, DVec3::new(0.5, 64.0, 4.5), (0, 64, 9));
        let ai = world.villager_mut(villager).unwrap().ai.as_mut().unwrap();
        ai.brain.memories.doors_to_close.set(vec![DOOR]);
        let heard = door_events(&mut world, &mut yard, 400, DOORS[1]);
        assert_eq!(
            heard,
            [(DOORS[1], centre(DOOR), Some(villager))],
            "it shuts the door it remembers behind it"
        );
        let mut yard = Yard::door_wall();
        let mut world = EntityWorld::default();
        world.set_dimension("minecraft:the_nether");
        let piglin = world.spawn_piglin(
            Piglin::new(PiglinKind::Piglin, DVec3::new(0.5, 64.0, 0.5)),
            false,
        );
        world.tick(&mut yard);
        let entity = world.piglin_mut(piglin).unwrap();
        let slowness = crate::effects::EffectInstance::new(MobEffect::Slowness, 10_000, 1);
        entity.effects.add(slowness);
        entity
            .brain
            .memories
            .walk_target
            .set(crate::brain_behaviors::WalkGoal {
                target: crate::brain_behaviors::Track::Block((0, 64, 7)),
                speed: 1.0,
                close_enough: 0,
            });
        let heard = door_events(&mut world, &mut yard, 1200, DOORS[1]);
        assert_eq!(
            heard,
            [
                (DOORS[0], centre(DOOR), Some(piglin)),
                (DOORS[1], centre(DOOR), Some(piglin))
            ]
        );
    }

    fn loved_gold() -> Arc<RecipeBook> {
        use std::io::Write;
        let jar = std::env::temp_dir().join(format!(
            "w3-piglin-tags-{}-{:?}.jar",
            std::process::id(),
            std::thread::current().id()
        ));
        let mut zip = zip::ZipWriter::new(std::fs::File::create(&jar).unwrap());
        zip.start_file(
            "data/minecraft/tags/item/piglin_loved.json",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        let loved = serde_json::json!({ "values": ["minecraft:gold_ingot"] });
        zip.write_all(loved.to_string().as_bytes()).unwrap();
        zip.finish().unwrap();
        let book = RecipeBook::from_jar(&jar).unwrap();
        let _ = std::fs::remove_file(&jar);
        Arc::new(book)
    }

    #[test]
    fn a_piglin_putting_on_a_helmet_posts_equip_and_taking_or_dropping_gold_unequip() {
        let mut yard = Yard::default();
        let mut world = EntityWorld::default();
        world.set_dimension("minecraft:the_nether");
        world.set_recipe_book(loved_gold());
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_piglin(Piglin::new(PiglinKind::Piglin, at), false);
        world.tick(&mut yard);
        world.take_game_events();
        let feet = world.body_of(id).unwrap().position;
        yard.drop("minecraft:golden_helmet", feet);
        world.tick(&mut yard);
        let worn = world
            .piglin_mut(id)
            .unwrap()
            .piglin
            .equipment(crate::piglin::HEAD)
            .cloned();
        assert_eq!(
            worn.map(|stack| stack.id),
            Some("minecraft:golden_helmet".to_owned())
        );
        let position = world.body_of(id).unwrap().position;
        assert_eq!(
            posted(&mut world, &EQUIPMENT),
            [(EQUIPMENT[0], position, Some(id))]
        );
        let mut inventory = minecraftoss_player::inventory::Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:gold_ingot", 2));
        assert!(
            world
                .piglin_admire_held(id, &mut inventory, 0, false)
                .is_some()
        );
        let position = world.body_of(id).unwrap().position;
        assert_eq!(
            posted(&mut world, &EQUIPMENT),
            [(EQUIPMENT[1], position, Some(id))],
            "given gold, which is no equipment"
        );
        let mut heard = Vec::new();
        for _ in 0..200 {
            world.tick(&mut yard);
            heard.extend(posted(&mut world, &EQUIPMENT));
            if world.piglin_mut(id).unwrap().piglin.offhand.is_none() {
                break;
            }
        }
        assert!(world.piglin_mut(id).unwrap().piglin.offhand.is_none());
        let position = world.body_of(id).unwrap().position;
        assert_eq!(
            heard,
            [(EQUIPMENT[1], position, Some(id))],
            "the gold leaves its hand"
        );
    }

    #[test]
    fn a_villager_shows_armour_as_equip_and_other_items_or_an_empty_hand_as_unequip() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_villager(crate::villager::Villager::new(at), true);
        let item = |id: &str| crate::trading::TradeItem {
            id: id.to_owned(),
            count: 1,
            components: Default::default(),
        };
        let entity = world.villager_mut(id).unwrap();
        let mut shown = Vec::new();
        for held in [
            Some(item("minecraft:iron_chestplate")),
            Some(item("minecraft:emerald")),
            Some(item("minecraft:emerald")),
            None,
            None,
        ] {
            let before = std::mem::replace(&mut entity.held_item, held);
            entity.held_item_changed(None, before.as_ref());
        }
        shown.extend(posted(&mut world, &EQUIPMENT));
        assert_eq!(
            shown,
            [
                (EQUIPMENT[0], at, Some(id)),
                (EQUIPMENT[1], at, Some(id)),
                (EQUIPMENT[1], at, Some(id))
            ],
            "the same item again or an empty hand again is nothing"
        );
    }

    #[test]
    fn a_witch_and_a_trader_post_unequip_as_they_start_and_finish_a_drink() {
        let mut yard = Yard::default();
        let mut world = EntityWorld::default();
        let mut witch = crate::witch::Witch::new(DVec3::new(0.5, 64.0, 0.5));
        witch.body.fire_ticks = 300;
        let witch = world.spawn_witch(witch, false);
        let trader = world.spawn_wandering_trader(
            crate::wandering_trader::WanderingTrader::new(DVec3::new(8.5, 64.0, 0.5)),
            false,
        );
        world.set_bright_outside(false);
        let (mut witch_drinks, mut trader_drinks) = (Vec::new(), Vec::new());
        for _ in 0..200 {
            let before = (
                world.witch_mut(witch).unwrap().witch.drinking.is_some(),
                world
                    .wandering_trader_mut(trader)
                    .unwrap()
                    .trader
                    .held
                    .is_some(),
            );
            world.tick(&mut yard);
            let after = (
                world.witch_mut(witch).unwrap().witch.drinking.is_some(),
                world
                    .wandering_trader_mut(trader)
                    .unwrap()
                    .trader
                    .held
                    .is_some(),
            );
            for (_, _, mob) in posted(&mut world, &[EQUIPMENT[1]]) {
                if mob == Some(witch) {
                    assert_ne!(before.0, after.0, "only as its hand changes");
                    witch_drinks.push(after.0);
                } else {
                    assert_eq!(mob, Some(trader));
                    assert_ne!(before.1, after.1, "only as its hand changes");
                    trader_drinks.push(after.1);
                }
            }
        }
        assert_eq!(witch_drinks[..2], [true, false], "{witch_drinks:?}");
        assert_eq!(trader_drinks, [true, false]);
    }

    #[test]
    fn saddles_horse_armour_and_carpets_put_on_post_equip_and_taken_off_unequip() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let held = |item: &str| {
            let mut inventory = minecraftoss_player::inventory::Inventory::default();
            inventory.slots[0] = Some(ItemStack::new(item, 1));
            inventory
        };
        let pig = world.spawn_pig_no_ai(crate::pig::Pig::new(at));
        let entity = world.pig_mut(pig).unwrap();
        entity
            .pig
            .interact_saddle(&mut held("minecraft:saddle"), 0, false);
        assert_eq!(
            posted(&mut world, &EQUIPMENT),
            [(EQUIPMENT[0], at, Some(pig))]
        );
        let strider = world.spawn_strider(crate::strider::Strider::new(at), true);
        let entity = world.strider_mut(strider).unwrap();
        entity
            .strider
            .interact_saddle(&mut held("minecraft:saddle"), 0, false);
        assert_eq!(
            posted(&mut world, &EQUIPMENT),
            [(EQUIPMENT[0], at, Some(strider))]
        );
        for (kind, body) in [
            (crate::horse::HorseKind::Horse, "minecraft:iron_horse_armor"),
            (crate::horse::HorseKind::Llama, "minecraft:red_carpet"),
        ] {
            let mut state = crate::horse::HorseState::new(kind);
            state.tame = true;
            let horse = world.spawn_horse(crate::cow::Cow::new(at), state, true);
            for item in ["minecraft:saddle", body] {
                world.horse_interact(horse, 0, &mut held(item), 0, false);
            }
            let saddled = kind.saddleable();
            let expected = if saddled { 2 } else { 1 };
            assert_eq!(
                posted(&mut world, &EQUIPMENT),
                vec![(EQUIPMENT[0], at, Some(horse)); expected],
                "{kind:?}"
            );
            world.set_mount_inventory(horse, false, None, Vec::new());
            assert_eq!(
                posted(&mut world, &EQUIPMENT),
                vec![(EQUIPMENT[1], at, Some(horse)); expected],
                "{kind:?} stripped in its inventory"
            );
        }
    }
}
