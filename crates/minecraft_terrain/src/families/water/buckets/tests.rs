use super::*;
use crate::bucket::{CAMPFIRE_DOUSE_SOUND, EVAPORATE_SOUND, evaporated};
use glam::DVec3;
use minecraftoss_entities::world::families::pests::endermites::Endermite;
use minecraftoss_entities::world::families::pests::silverfish::Silverfish;
use minecraftoss_player::Block;
use minecraftoss_player::inventory::Inventory;
use minecraftoss_world::level::mob_bucket::{EMPTY_FISH, FILL_FISH, entity_data, no_components};
use serde_json::json;
use std::collections::{BTreeMap, HashMap};

static SILVERFISH_BUCKET: MobBucket = MobBucket {
    item: "minecraft:silverfish_bucket",
    kind: "minecraft:silverfish",
    water: true,
    empty_sound: EMPTY_FISH,
    fill_sound: FILL_FISH,
    keys: &[],
    components: no_components,
};

static ENDERMITE_BUCKET: MobBucket = MobBucket {
    item: "minecraft:endermite_bucket",
    kind: "minecraft:endermite",
    water: false,
    empty_sound: "minecraft:item.bucket.empty_sulfur_cube",
    fill_sound: "minecraft:item.bucket.fill_sulfur_cube",
    keys: &[],
    components: no_components,
};

impl Bucketable for Silverfish {
    fn bucket(_kind: &str) -> Option<&'static MobBucket> {
        Some(&SILVERFISH_BUCKET)
    }

    fn bucket_components(mob: &FamilyMob<Self>) -> Vec<(&'static str, Value)> {
        vec![("minecraft:silverfish/size", json!(mob.core.kind))]
    }
}

impl Bucketable for Endermite {
    const PICKUP_ITEM: &'static str = BUCKET;

    fn bucket(_kind: &str) -> Option<&'static MobBucket> {
        Some(&ENDERMITE_BUCKET)
    }
}

#[derive(Default)]
struct Blocks(HashMap<Pos, Block>);

impl World for Blocks {
    fn block(&self, pos: Pos) -> Option<Block> {
        self.0.get(&pos).cloned()
    }

    fn set_block(&mut self, pos: Pos, block: Option<Block>) {
        match block {
            Some(block) => self.0.insert(pos, block),
            None => self.0.remove(&pos),
        };
    }
}

#[derive(Default)]
struct Rules {
    evaporates: bool,
}

impl Materials for Rules {
    fn can_be_replaced(&self, block: &Block) -> bool {
        matches!(
            block.id.as_str(),
            "minecraft:water" | "minecraft:short_grass"
        )
    }

    fn sturdy_top(&self, block: &Block) -> bool {
        block.id == "minecraft:stone"
    }

    fn flammable(&self, _block: &Block) -> bool {
        false
    }

    fn water_evaporates(&self) -> bool {
        self.evaporates
    }
}

fn looking_down(below: Block) -> (Player, Blocks) {
    let mut world = Blocks::default();
    world.set_block((0, 0, 0), Some(below));
    let mut player = Player::new(DVec3::new(0.5, 2.0, 0.5));
    player.pitch = 90.0;
    (player, world)
}

fn stone() -> Block {
    Block::new("minecraft:stone")
}

fn held(id: &str) -> ItemStack {
    bucket_stack(id)
}

fn no_random() -> f32 {
    0.5
}

fn emptied(used: &ItemUse) -> &MobBucketUse {
    match &used.change {
        Change::MobBucket(bucket) => bucket,
        other => panic!("not a mob bucket: {other:?}"),
    }
}

fn events(used: &ItemUse) -> Vec<&'static str> {
    used.sounds.iter().map(|sound| sound.event).collect()
}

#[test]
fn a_fish_bucket_aims_like_water_with_its_own_splash() {
    let rules = Rules::default();
    let (mut player, world) = looking_down(stone());
    let cod = crate::bucket::bucket_use(
        &player,
        &world,
        &rules,
        &held("minecraft:cod_bucket"),
        no_random,
    )
    .unwrap();
    assert_eq!(
        *emptied(&cod),
        MobBucketUse {
            pos: (0, 1, 0),
            at: (0, 1, 0),
            item: "minecraft:cod_bucket",
            components: None,
        }
    );
    assert_eq!(cod.sounds, vec![Sound::at_block(EMPTY_FISH, (0, 1, 0))]);
    let result = cod.result.clone().unwrap();
    assert_eq!((result.id.as_str(), result.max), (BUCKET, 16));
    assert!(!cod.consumed);
    for (item, splash) in [
        ("minecraft:salmon_bucket", EMPTY_FISH),
        ("minecraft:tropical_fish_bucket", EMPTY_FISH),
        ("minecraft:pufferfish_bucket", EMPTY_FISH),
        (
            "minecraft:axolotl_bucket",
            "minecraft:item.bucket.empty_axolotl",
        ),
        (
            "minecraft:tadpole_bucket",
            "minecraft:item.bucket.empty_tadpole",
        ),
        (
            "minecraft:sulfur_cube_bucket",
            "minecraft:item.bucket.empty_sulfur_cube",
        ),
    ] {
        let used = bucket_use(&player, &world, &rules, &held(item), no_random).unwrap();
        assert_eq!(events(&used), [splash], "{item}");
        assert_eq!(emptied(&used).item, item);
    }
    let stairs = Block::new("minecraft:oak_stairs").with("waterlogged", "false");
    let (_, logged) = looking_down(stairs);
    let into = bucket_use(
        &player,
        &logged,
        &rules,
        &held("minecraft:cod_bucket"),
        no_random,
    )
    .unwrap();
    assert_eq!(
        (emptied(&into).pos, emptied(&into).at),
        ((0, 0, 0), (0, 0, 0)),
        "water and fish go into the waterloggable block"
    );
    let cube = bucket_use(
        &player,
        &logged,
        &rules,
        &held("minecraft:sulfur_cube_bucket"),
        no_random,
    )
    .unwrap();
    assert_eq!(
        (emptied(&cube).pos, emptied(&cube).at),
        ((0, 1, 0), (0, 1, 0)),
        "the cube, with no water, goes against the clicked face"
    );
    player.crouching = true;
    let sneaking = bucket_use(
        &player,
        &logged,
        &rules,
        &held("minecraft:cod_bucket"),
        no_random,
    )
    .unwrap();
    assert_eq!(
        (emptied(&sneaking).pos, emptied(&sneaking).at),
        ((0, 1, 0), (0, 0, 0)),
        "sneaking pours on top, and the fish still comes out where the bucket aimed"
    );
}

#[test]
fn a_fish_bucket_hisses_where_water_evaporates_and_douses_a_campfire() {
    let (player, world) = looking_down(stone());
    let hot = Rules { evaporates: true };
    let mut rolls = [0.75, 0.25].into_iter();
    let boiled =
        crate::bucket::bucket_use(&player, &world, &hot, &held("minecraft:cod_bucket"), || {
            rolls.next().unwrap()
        })
        .unwrap();
    assert_eq!(
        events(&boiled),
        [EVAPORATE_SOUND],
        "only the hiss, no splash"
    );
    assert_eq!(evaporated(&boiled), Some((0, 1, 0)), "with the smoke");
    assert_eq!(emptied(&boiled).at, (0, 1, 0), "the fish still comes out");
    let campfire = Block::new("minecraft:campfire")
        .with("lit", "true")
        .with("waterlogged", "false");
    let (player, world) = looking_down(campfire);
    let doused = bucket_use(
        &player,
        &world,
        &Rules::default(),
        &held("minecraft:cod_bucket"),
        no_random,
    )
    .unwrap();
    assert_eq!(events(&doused), [CAMPFIRE_DOUSE_SOUND, EMPTY_FISH]);
    assert_eq!(evaporated(&doused), None);
}

#[test]
fn a_mob_bucket_sends_its_components_and_other_items_pass() {
    let (player, world) = looking_down(stone());
    let rules = Rules::default();
    let mut named = held("minecraft:tropical_fish_bucket");
    named.components = Some(serde_json::json!({
        "minecraft:bucket_entity_data": {"Health": 2.5, "NoAI": true},
        "minecraft:custom_name": "Nemo",
    }));
    let used = bucket_use(&player, &world, &rules, &named, no_random).unwrap();
    let components = emptied(&used).components.clone().unwrap();
    let data = components.get("minecraft:bucket_entity_data").unwrap();
    assert_eq!(data.get("Health"), Some(&Tag::Double(2.5)));
    assert_eq!(data.get("NoAI"), Some(&Tag::Byte(1)));
    assert_eq!(
        components.get("minecraft:custom_name"),
        Some(&Tag::String("Nemo".to_owned()))
    );
    for other in [
        "minecraft:water_bucket",
        "minecraft:bucket",
        "minecraft:milk_bucket",
    ] {
        assert_eq!(
            bucket_use(&player, &world, &rules, &held(other), no_random),
            None,
            "{other}"
        );
    }
    let mut far = player.clone();
    far.pos = DVec3::new(0.5, 9.0, 0.5);
    assert_eq!(
        bucket_use(
            &far,
            &world,
            &rules,
            &held("minecraft:cod_bucket"),
            no_random
        ),
        None,
        "nothing in reach"
    );
}

fn spawned<S: Bucketable>(world: &mut EntityWorld, kind: &'static str, x: f64) -> u64 {
    let mob = S::spawn(kind, DVec3::new(x, 64.0, 0.5)).unwrap();
    world.spawn_family(mob)
}

fn holding(item: &str) -> Inventory {
    let mut inventory = Inventory::default();
    inventory.slots[0] = Some(bucket_stack(item));
    inventory
}

fn use_on<S: Bucketable>(
    world: &mut EntityWorld,
    id: u64,
    inventory: &mut Inventory,
    infinite: bool,
) -> (bool, MobOutcome) {
    let mut actor = Actor {
        player: 1,
        inventory,
        selected: 0,
        infinite,
        entity_loot: None,
        shearing_loot: None,
    };
    let mut outcome = MobOutcome::default();
    let picked = pickup::<S>(world, id, &mut actor, &mut outcome);
    (picked, outcome)
}

fn removed<S: Bucketable>(world: &EntityWorld, id: u64) -> bool {
    world
        .family::<S>()
        .iter()
        .any(|mob| mob.core.id == id && mob.core.removed)
}

fn held_id(inventory: &Inventory) -> Option<&str> {
    inventory.slots[0].as_ref().map(|stack| stack.id.as_str())
}

#[test]
fn a_picked_up_mob_keeps_its_health_and_vanishes_without_dying() {
    let mut world = EntityWorld::default();
    let id = spawned::<Silverfish>(&mut world, "minecraft:silverfish", 0.5);
    let mob = world.family_mob_mut::<Silverfish>(id).unwrap();
    mob.core.health = 5.0;
    mob.core.no_ai = true;
    mob.core.persistent = true;
    let position = mob.core.body.position;
    let mut inventory = holding(WATER_BUCKET);
    let (picked, outcome) = use_on::<Silverfish>(&mut world, id, &mut inventory, false);
    assert!(picked && outcome.inventory_changed);
    let filled = inventory.slots[0].clone().expect("the filled bucket");
    assert_eq!(
        (filled.id.as_str(), filled.count, filled.max),
        ("minecraft:silverfish_bucket", 1, 1)
    );
    assert_eq!(
        filled.components,
        Some(json!({
            "minecraft:bucket_entity_data": {"Health": 5.0, "NoAI": 1, "PersistenceRequired": 1},
            "minecraft:silverfish/size": "minecraft:silverfish",
        }))
    );
    let fill = MobSound {
        event: FILL_FISH.to_owned(),
        position,
        volume: 1.0,
        pitch: 1.0,
        category: "hostile_volume",
    };
    assert_eq!(outcome.sounds, [fill]);
    assert!(outcome.drops.is_empty());
    assert!(removed::<Silverfish>(&world, id));
    assert!(
        world.take_deaths().is_empty(),
        "no death, loot or experience"
    );
    let mut another = holding(WATER_BUCKET);
    let (again, _) = use_on::<Silverfish>(&mut world, id, &mut another, false);
    assert!(!again, "a picked-up mob is gone");
    let components = filled
        .components
        .as_ref()
        .and_then(crate::item_nbt::components_tag);
    let poured: BTreeMap<String, Tag> = entity_data(&SILVERFISH_BUCKET, components.as_ref())
        .into_iter()
        .collect();
    assert_eq!(poured.get("Health"), Some(&Tag::Float(5.0)));
    assert_eq!(poured.get("NoAI"), Some(&Tag::Byte(1)));
    assert_eq!(poured.get("PersistenceRequired"), Some(&Tag::Byte(1)));
    assert_eq!(poured.get("FromBucket"), Some(&Tag::Byte(1)));
    let plain = spawned::<Silverfish>(&mut world, "minecraft:silverfish", 3.5);
    let mut fresh = holding(WATER_BUCKET);
    assert!(use_on::<Silverfish>(&mut world, plain, &mut fresh, false).0);
    let saved = fresh.slots[0]
        .as_ref()
        .and_then(|stack| stack.components.clone());
    assert_eq!(
        saved.map(|components| components["minecraft:bucket_entity_data"].clone()),
        Some(json!({"Health": 8.0})),
        "only set flags are saved, and the health always"
    );
}

#[test]
fn only_the_family_s_pickup_item_picks_a_mob_up() {
    let mut world = EntityWorld::default();
    let silverfish = spawned::<Silverfish>(&mut world, "minecraft:silverfish", 0.5);
    let endermite = spawned::<Endermite>(&mut world, "minecraft:endermite", 4.5);
    for item in [BUCKET, "minecraft:lava_bucket", "minecraft:cod_bucket"] {
        let mut inventory = holding(item);
        let (picked, outcome) = use_on::<Silverfish>(&mut world, silverfish, &mut inventory, false);
        assert!(!picked && outcome.sounds.is_empty(), "{item}");
        assert_eq!(held_id(&inventory), Some(item));
    }
    let mut bare = Inventory::default();
    assert!(!use_on::<Silverfish>(&mut world, silverfish, &mut bare, false).0);
    let mut water = holding(WATER_BUCKET);
    assert!(
        !use_on::<Endermite>(&mut world, endermite, &mut water, false).0,
        "this family takes an empty bucket"
    );
    let mut empty = holding(BUCKET);
    let (picked, outcome) = use_on::<Endermite>(&mut world, endermite, &mut empty, false);
    assert!(picked);
    assert_eq!(held_id(&empty), Some("minecraft:endermite_bucket"));
    assert_eq!(
        outcome.sounds[0].event,
        "minecraft:item.bucket.fill_sulfur_cube"
    );
    assert!(use_on::<Silverfish>(&mut world, silverfish, &mut water, false).0);
    assert_eq!(held_id(&water), Some("minecraft:silverfish_bucket"));
    let dead = spawned::<Silverfish>(&mut world, "minecraft:silverfish", 8.5);
    world
        .family_mob_mut::<Silverfish>(dead)
        .unwrap()
        .core
        .health = 0.0;
    let mut late = holding(WATER_BUCKET);
    assert!(
        !use_on::<Silverfish>(&mut world, dead, &mut late, false).0,
        "a dead mob cannot be scooped"
    );
}

#[test]
fn creative_keeps_the_water_bucket_and_adds_one_fish_bucket() {
    let mut world = EntityWorld::default();
    let mut inventory = holding(WATER_BUCKET);
    for x in [0.5, 2.5] {
        let id = spawned::<Silverfish>(&mut world, "minecraft:silverfish", x);
        let (picked, outcome) = use_on::<Silverfish>(&mut world, id, &mut inventory, true);
        assert!(picked && outcome.drops.is_empty());
        assert!(removed::<Silverfish>(&world, id));
    }
    let hand = inventory.slots[0]
        .as_ref()
        .map(|stack| (stack.id.as_str(), stack.count));
    assert_eq!(hand, Some((WATER_BUCKET, 1)));
    let buckets = inventory
        .slots
        .iter()
        .flatten()
        .filter(|stack| stack.id == "minecraft:silverfish_bucket")
        .count();
    assert_eq!(buckets, 2, "every pickup adds a bucket");
    let mut full = holding(WATER_BUCKET);
    for slot in full.slots.iter_mut().skip(1) {
        *slot = Some(ItemStack::new("minecraft:stone", 64));
    }
    let id = spawned::<Silverfish>(&mut world, "minecraft:silverfish", 6.5);
    let (picked, outcome) = use_on::<Silverfish>(&mut world, id, &mut full, true);
    assert!(picked);
    let dropped: Vec<(&str, DVec3)> = outcome
        .drops
        .iter()
        .map(|(stack, at)| (stack.id.as_str(), *at))
        .collect();
    assert_eq!(
        dropped,
        [("minecraft:silverfish_bucket", DVec3::new(6.5, 64.0, 0.5))],
        "with no room it drops where the mob was"
    );
}
