use super::*;
use crate::bucket::{CAMPFIRE_DOUSE_SOUND, EVAPORATE_SOUND, evaporated};
use glam::DVec3;
use minecraftoss_player::Block;
use minecraftoss_world::level::mob_bucket::EMPTY_FISH;
use std::collections::HashMap;

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
