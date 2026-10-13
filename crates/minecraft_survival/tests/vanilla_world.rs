use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use minecraft_commands::{CommandServer, PlayerId};
use minecraft_net::Dimension;
use minecraft_survival::data::GameData;
use minecraft_survival::player::Player;
use minecraft_survival::player_nbt::{GlobalPos, Place, Respawn};
use minecraft_survival::save::{OpenedWorld, open_world};
use minecraft_survival::server::{Joining, join};
use minecraft_terrain::containers::{Containers, Opened, block_entity_kind};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_mode::{GameMode, PlayerMode};
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::level_storage::WeatherData;
use minecraft_terrain::scene::Block;
use minecraftoss_core::nbt::{self, Tag};
use minecraftoss_core::registries::DataPaths;
use minecraftoss_core::{BlockPos, BlockStateId, ChunkPos, Registries};
use minecraftoss_generator::terrain::TerrainGenerator;
use minecraftoss_player::Difficulty;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::survival::EffectKind;
use minecraftoss_world::chunk_map::{ChunkMap, WorldGen};
use minecraftoss_world::storage::ChunkStorage;
use serde_json::json;

const SEED: i64 = 20_263;
const SPAWN: (i32, i32, i32) = (8, 70, 8);
const CHEST: (i32, i32, i32) = (3, 250, 3);
const FURNACE: (i32, i32, i32) = (5, 250, 3);
const ENDER: (i32, i32, i32) = (7, 250, 3);
const CHEST_STATE: &str = "minecraft:chest[facing=north,type=single,waterlogged=false]";
const FURNACE_STATE: &str = "minecraft:furnace[facing=north,lit=true]";

struct TestDir(PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

struct Vanilla {
    registries: Arc<Registries>,
    worldgen: Arc<WorldGen>,
    data: GameData,
}

fn vanilla() -> Option<Vanilla> {
    let root = PathBuf::from(std::env::var_os("MINECRAFTOSS_ROOT")?);
    let registries = Arc::new(Registries::load(&DataPaths::discover().ok()?).ok()?);
    registries.block_entities.as_ref()?;
    let jar = root.join("client.jar");
    if !jar.is_file() {
        return None;
    }
    let data = GameData::load(Some(&root), Some(jar));
    let generator = TerrainGenerator::overworld(registries.clone(), SEED).ok()?;
    let worldgen = Arc::new(WorldGen::new(Arc::new(generator)).ok()?);
    Some(Vanilla {
        registries,
        worldgen,
        data,
    })
}

fn chunk_map(vanilla: &Vanilla, world: &Path) -> ChunkMap {
    let storage = ChunkStorage::new(
        world,
        Dimension::Overworld.dimension_type(),
        vanilla.registries.clone(),
        -64,
        384,
    );
    ChunkMap::with_storage(vanilla.worldgen.clone(), 2, 2, Some(storage))
}

fn steve() -> minecraft_commands::server::Profile {
    let mut chat = CommandServer::new(PlayerId(0));
    chat.join(PlayerId(0), "Steve");
    chat.profile(PlayerId(0)).unwrap().clone()
}

fn joined(opened: &OpenedWorld, data: &GameData, rules: &GameRules) -> Player {
    let profile = steve();
    join(
        opened.save.as_ref(),
        &data.recipes,
        rules,
        Joining {
            profile: &profile,
            spawn: [8.5, 70.0, 8.5],
            chosen_mode: None,
            kit: Some("minecraft:dirt*5"),
            carried: None,
        },
    )
}

fn block(state: &str) -> Block {
    let (id, properties) = state.split_once('[').unwrap();
    properties
        .trim_end_matches(']')
        .split(',')
        .fold(Block::new(id), |block, pair| {
            let (key, value) = pair.split_once('=').unwrap();
            block.with(key, value)
        })
}

fn opened(pos: (i32, i32, i32), state: &str) -> Opened {
    Opened::new(pos, &block(state), None).unwrap()
}

fn kind_name(tag: &Tag) -> &'static str {
    match tag {
        Tag::Byte(_) => "byte",
        Tag::Short(_) => "short",
        Tag::Int(_) => "int",
        Tag::Long(_) => "long",
        Tag::Float(_) => "float",
        Tag::Double(_) => "double",
        Tag::ByteArray(_) => "byte array",
        Tag::String(_) => "string",
        Tag::List(_) => "list",
        Tag::Compound(_) => "compound",
        Tag::IntArray(_) => "int array",
        Tag::LongArray(_) => "long array",
    }
}

fn kinds(tag: &Tag) -> BTreeMap<&str, &'static str> {
    tag.as_compound()
        .unwrap()
        .iter()
        .map(|(key, value)| (key.as_str(), kind_name(value)))
        .collect()
}

fn expect_kinds(tag: &Tag, expected: &[(&str, &'static str)], what: &str) {
    let expected: BTreeMap<&str, &'static str> = expected.iter().copied().collect();
    assert_eq!(kinds(tag), expected, "{what}");
}

fn list_kinds<'a>(tag: &'a Tag, what: &str) -> &'a [Tag] {
    let list = tag.as_list().unwrap();
    assert!(!list.is_empty(), "{what} is empty");
    list
}

fn read_nbt(path: &Path) -> Tag {
    nbt::read(&std::fs::read(path).unwrap()).unwrap()
}

fn stack(data: &GameData, id: &str, count: u8) -> ItemStack {
    data.recipes.stack(id, count)
}

fn sword(data: &GameData) -> ItemStack {
    let mut sword = stack(data, "minecraft:diamond_sword", 1);
    sword.components = Some(json!({
        "minecraft:enchantments": {"minecraft:sharpness": 5},
        "minecraft:damage": 30
    }));
    sword
}

fn gun(data: &GameData) -> ItemStack {
    let mut gun = stack(data, "iw4l:m4", 1);
    gun.components = Some(json!({
        "minecraft:custom_data": {"iw4l": {"attachments": ["reflex"], "clip": 27}}
    }));
    gun
}

fn check_gun_kinds(item: &Tag, what: &str) {
    assert_eq!(
        item.get("id").and_then(Tag::as_str),
        Some("minecraft:carrot_on_a_stick"),
        "{what}"
    );
    let components = item.get("components").unwrap();
    expect_kinds(
        components,
        &[
            ("minecraft:custom_data", "compound"),
            ("minecraft:item_name", "compound"),
        ],
        what,
    );
    expect_kinds(
        components
            .get("minecraft:custom_data")
            .and_then(|custom| custom.get("iw4l"))
            .unwrap(),
        &[("attachments", "list"), ("clip", "int"), ("id", "string")],
        what,
    );
    expect_kinds(
        components.get("minecraft:item_name").unwrap(),
        &[("fallback", "string"), ("translate", "string")],
        what,
    );
}

fn held(slots: &[Option<ItemStack>]) -> Vec<(usize, String, u8)> {
    slots
        .iter()
        .enumerate()
        .filter_map(|(slot, stack)| {
            let stack = stack.as_ref()?;
            Some((slot, stack.id.clone(), stack.count))
        })
        .collect()
}

fn fill_player(player: &mut Player, data: &GameData) {
    let slots = &mut player.inventory.slots;
    slots.fill(None);
    slots[0] = Some(sword(data));
    slots[1] = Some(gun(data));
    slots[2] = Some(stack(data, "iw4l:frag_grenade", 16));
    slots[4] = Some(stack(data, "minecraft:golden_apple", 3));
    slots[9] = Some(stack(data, "minecraft:raw_iron", 3));
    slots[10] = Some(stack(data, "minecraft:coal", 2));
    slots[11] = Some(stack(data, "minecraft:cobblestone", 64));
    slots[20] = Some(stack(data, "minecraft:emerald", 12));
    slots[35] = Some(stack(data, "minecraft:torch", 16));
    slots[36] = Some(stack(data, "minecraft:diamond_boots", 1));
    slots[37] = Some(stack(data, "minecraft:diamond_leggings", 1));
    slots[38] = Some(stack(data, "minecraft:diamond_chestplate", 1));
    slots[39] = Some(stack(data, "minecraft:diamond_helmet", 1));
    slots[40] = Some(stack(data, "minecraft:shield", 1));
    player.selected = 4;
    player.mode = PlayerMode::new(GameMode::Survival);
    let status = &mut player.survival.status;
    status.health = 13.5;
    status.food.level = 15;
    status.food.saturation = 1.5;
    status.food.exhaustion = 2.25;
    status.experience_level = 7;
    status.experience_progress = 0.375;
    status.total_experience = 121;
    status.air = 200;
    status.add_effect(EffectKind::Regeneration, 400, 1);
    status.add_effect(EffectKind::Absorption, 2400, 0);
    status.add_effect(EffectKind::FireResistance, u32::MAX, 0);
    status.absorption = 4.0;
    player.survival.score = 121;
    player.place = Some(Place {
        dimension: Dimension::Overworld,
        feet: [120.25, 81.0, -64.75],
        yaw: -135.5,
        pitch: 22.25,
        motion: [0.0, -0.0784000015258789, 0.0],
        on_ground: true,
        fall_distance: 0.0,
    });
    player.set_respawn(Some(Respawn {
        at: GlobalPos {
            dimension: Dimension::Overworld,
            pos: (100, 64, -50),
        },
        yaw: 90.0,
        pitch: 0.0,
        forced: true,
    }));
    player.portal_cooldown = 7;
    player.seen_credits = true;
}

fn stats_of(player: &Player) -> (f32, u8, f32, f32, u32, f32, u32, i32, f32, i32) {
    let status = &player.survival.status;
    (
        status.health,
        status.food.level,
        status.food.saturation,
        status.food.exhaustion,
        status.experience_level,
        status.experience_progress,
        status.total_experience,
        status.air,
        status.absorption,
        player.survival.score,
    )
}

fn effects_of(player: &Player) -> Vec<(EffectKind, u32, u8)> {
    [
        EffectKind::Regeneration,
        EffectKind::Absorption,
        EffectKind::FireResistance,
        EffectKind::Poison,
    ]
    .into_iter()
    .filter_map(|kind| {
        let effect = player.survival.status.effect(kind)?;
        Some((kind, effect.duration, effect.amplifier))
    })
    .collect()
}

fn check_player_file(path: &Path) {
    let file = read_nbt(path);
    expect_kinds(
        &file,
        &[
            ("AbsorptionAmount", "float"),
            ("Air", "short"),
            ("DataVersion", "int"),
            ("Dimension", "string"),
            ("EnderItems", "list"),
            ("Fire", "short"),
            ("Health", "float"),
            ("Inventory", "list"),
            ("Motion", "list"),
            ("OnGround", "byte"),
            ("PortalCooldown", "int"),
            ("Pos", "list"),
            ("Rotation", "list"),
            ("Score", "int"),
            ("SelectedItemSlot", "int"),
            ("SleepTimer", "short"),
            ("UUID", "int array"),
            ("XpLevel", "int"),
            ("XpP", "float"),
            ("XpTotal", "int"),
            ("abilities", "compound"),
            ("active_effects", "list"),
            ("attributes", "list"),
            ("equipment", "compound"),
            ("fall_distance", "double"),
            ("foodExhaustionLevel", "float"),
            ("foodLevel", "int"),
            ("foodSaturationLevel", "float"),
            ("foodTickTimer", "int"),
            ("playerGameType", "int"),
            ("previousPlayerGameType", "int"),
            ("recipeBook", "compound"),
            ("respawn", "compound"),
            ("seenCredits", "byte"),
        ],
        "the player file",
    );
    expect_kinds(
        file.get("recipeBook").unwrap(),
        &[("recipes", "list"), ("toBeDisplayed", "list")],
        "the recipe book",
    );
    for item in list_kinds(file.get("Inventory").unwrap(), "Inventory")
        .iter()
        .chain(list_kinds(file.get("EnderItems").unwrap(), "EnderItems"))
    {
        let mut expected = vec![("Slot", "byte"), ("count", "int"), ("id", "string")];
        if item.get("components").is_some() {
            expected.push(("components", "compound"));
        }
        expect_kinds(item, &expected, "an item with its slot");
    }
    let sword = list_kinds(file.get("Inventory").unwrap(), "Inventory")
        .iter()
        .find(|item| item.get("Slot") == Some(&Tag::Byte(0)))
        .unwrap();
    expect_kinds(
        sword.get("components").unwrap(),
        &[
            ("minecraft:damage", "int"),
            ("minecraft:enchantments", "compound"),
        ],
        "the sword's components",
    );
    let gun = list_kinds(file.get("Inventory").unwrap(), "Inventory")
        .iter()
        .find(|item| item.get("Slot") == Some(&Tag::Byte(1)))
        .unwrap();
    check_gun_kinds(gun, "the gun in the player file");
    let grenades = list_kinds(file.get("Inventory").unwrap(), "Inventory")
        .iter()
        .find(|item| item.get("Slot") == Some(&Tag::Byte(2)))
        .unwrap();
    assert_eq!(
        grenades.get("id").and_then(Tag::as_str),
        Some("minecraft:poisonous_potato")
    );
    expect_kinds(
        grenades.get("components").unwrap(),
        &[
            ("!minecraft:consumable", "compound"),
            ("!minecraft:food", "compound"),
            ("minecraft:custom_data", "compound"),
            ("minecraft:item_name", "compound"),
        ],
        "the grenades' components",
    );
    let equipment = file.get("equipment").unwrap();
    let names: Vec<&str> = equipment
        .as_compound()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(names, ["chest", "feet", "head", "legs", "offhand"]);
    for piece in equipment.as_compound().unwrap().values() {
        expect_kinds(piece, &[("count", "int"), ("id", "string")], "equipment");
    }
    expect_kinds(
        file.get("abilities").unwrap(),
        &[
            ("flySpeed", "float"),
            ("flying", "byte"),
            ("instabuild", "byte"),
            ("invulnerable", "byte"),
            ("mayBuild", "byte"),
            ("mayfly", "byte"),
            ("walkSpeed", "float"),
        ],
        "abilities",
    );
    for effect in list_kinds(file.get("active_effects").unwrap(), "active_effects") {
        expect_kinds(
            effect,
            &[
                ("ambient", "byte"),
                ("amplifier", "byte"),
                ("duration", "int"),
                ("id", "string"),
                ("show_icon", "byte"),
                ("show_particles", "byte"),
            ],
            "an effect",
        );
    }
    for attribute in list_kinds(file.get("attributes").unwrap(), "attributes") {
        expect_kinds(
            attribute,
            &[("base", "double"), ("id", "string"), ("modifiers", "list")],
            "an attribute",
        );
    }
    for (key, kind, count) in [
        ("Pos", "double", 3),
        ("Motion", "double", 3),
        ("Rotation", "float", 2),
    ] {
        let values = file.get(key).unwrap().as_list().unwrap();
        assert_eq!(values.len(), count, "{key}");
        assert!(values.iter().all(|value| kind_name(value) == kind), "{key}");
    }
    expect_kinds(
        file.get("respawn").unwrap(),
        &[
            ("dimension", "string"),
            ("forced", "byte"),
            ("pitch", "float"),
            ("pos", "int array"),
            ("yaw", "float"),
        ],
        "respawn",
    );
}

fn check_world_files(world: &Path, uuid: &str) {
    for file in [
        "level.dat",
        "session.lock",
        "data/minecraft/world_gen_settings.dat",
        "data/minecraft/world_clocks.dat",
        "data/minecraft/game_rules.dat",
        "data/minecraft/weather.dat",
        "dimensions/minecraft/overworld/region/r.0.0.mca",
    ] {
        assert!(world.join(file).is_file(), "{file}");
    }
    let level = read_nbt(&world.join("level.dat"));
    let data = level.get("Data").unwrap();
    let wanted = [
        ("DataVersion", "int"),
        ("GameType", "int"),
        ("LevelName", "string"),
        ("Time", "long"),
        ("allowCommands", "byte"),
        ("difficulty_settings", "compound"),
        ("initialized", "byte"),
        ("singleplayer_uuid", "int array"),
        ("spawn", "compound"),
        ("version", "int"),
    ];
    for (key, kind) in wanted {
        assert_eq!(data.get(key).map(kind_name), Some(kind), "level.dat {key}");
    }
    expect_kinds(
        data.get("spawn").unwrap(),
        &[
            ("dimension", "string"),
            ("pitch", "float"),
            ("pos", "int array"),
            ("yaw", "float"),
        ],
        "level.dat spawn",
    );
    let weather = read_nbt(&world.join("data/minecraft/weather.dat"));
    expect_kinds(
        &weather,
        &[("DataVersion", "int"), ("data", "compound")],
        "weather.dat",
    );
    expect_kinds(
        weather.get("data").unwrap(),
        &[
            ("clear_weather_time", "int"),
            ("rain_time", "int"),
            ("raining", "byte"),
            ("thunder_time", "int"),
            ("thundering", "byte"),
        ],
        "the weather",
    );
    check_player_file(&world.join("players/data").join(format!("{uuid}.dat")));
}

fn check_block_entities(vanilla: &Vanilla, chest: &Tag, furnace: &Tag) {
    let catalog = vanilla.registries.block_entities.as_ref().unwrap();
    for (block, pos, tag) in [
        ("minecraft:chest", CHEST, chest),
        ("minecraft:furnace", FURNACE, furnace),
    ] {
        assert_eq!(
            catalog.load_and_save(block, pos, tag).as_ref(),
            Some(tag),
            "vanilla loads and saves the {block} the same"
        );
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(block));
        for (key, value) in [("x", pos.0), ("y", pos.1), ("z", pos.2)] {
            assert_eq!(tag.get(key), Some(&Tag::Int(value)), "{block} {key}");
        }
        for item in list_kinds(tag.get("Items").unwrap(), "Items") {
            let mut expected = vec![("Slot", "byte"), ("count", "int"), ("id", "string")];
            if item.get("components").is_some() {
                expected.push(("components", "compound"));
            }
            expect_kinds(item, &expected, "a container item");
        }
    }
    for (key, kind) in [
        ("cooking_time_spent", "int"),
        ("cooking_total_time", "int"),
        ("lit_time_remaining", "int"),
        ("lit_total_time", "int"),
        ("speed_multiplier", "float"),
        ("RecipesUsed", "compound"),
    ] {
        assert_eq!(furnace.get(key).map(kind_name), Some(kind), "furnace {key}");
    }
    let used = furnace
        .get("RecipesUsed")
        .and_then(Tag::as_compound)
        .unwrap();
    assert!(!used.is_empty() && used.values().all(|count| kind_name(count) == "int"));
}

#[test]
fn a_saved_world_comes_back_whole_with_vanilla_keys_and_tag_types() {
    let Some(vanilla) = vanilla() else {
        return;
    };
    let data = &vanilla.data;
    let saves = TestDir(std::env::temp_dir().join(format!(
        "minecraft_survival-vanilla-world-{}",
        std::process::id()
    )));
    let _ = std::fs::remove_dir_all(&saves.0);
    let rules = GameRules::default();
    let weather = WeatherData {
        clear_weather_time: 0,
        rain_time: 4_321,
        thunder_time: 1_234,
        raining: true,
        thundering: true,
    };
    let day = DayCycle {
        ticks: 13_500.0,
        paused: false,
    };

    let mut first = open_world(&saves.0, Some("compat"), Some(&SEED.to_string()), 9);
    let world = first
        .save
        .as_ref()
        .expect("a world folder")
        .path()
        .to_owned();
    let mut player = joined(&first, data, &rules);
    let uuid = player.uuid.0.clone();
    first.save.as_mut().unwrap().set_host(&uuid);
    fill_player(&mut player, data);

    let mut containers = Containers::default();
    containers.set_book(data.stations.clone());
    let chest = opened(CHEST, CHEST_STATE);
    let furnace = opened(FURNACE, FURNACE_STATE);
    let ender = opened(
        ENDER,
        "minecraft:ender_chest[facing=north,waterlogged=false]",
    );
    containers.quick_move_from_player(&chest, &mut player.inventory, 11);
    player.inventory.cursor = Some(sword(data));
    containers.click(&chest, &mut player.inventory, 13, false, false, 0.0);
    player.inventory.cursor = Some(gun(data));
    containers.click(&chest, &mut player.inventory, 14, false, false, 0.0);
    player.inventory.cursor = Some(stack(data, "minecraft:bread", 6));
    containers.click(&chest, &mut player.inventory, 26, false, false, 0.0);
    containers.quick_move_from_player(&furnace, &mut player.inventory, 9);
    containers.quick_move_from_player(&furnace, &mut player.inventory, 10);
    containers.with_ender(&mut player.ender, |containers| {
        containers.quick_move_from_player(&ender, &mut player.inventory, 20);
    });
    for _ in 0..250 {
        containers.tick(&data.recipes);
    }
    let cooking = containers.oven(FURNACE).unwrap().data();
    assert!(cooking[2] > 0, "the furnace is cooking: {cooking:?}");
    assert_eq!(
        containers
            .oven(FURNACE)
            .unwrap()
            .recipes_used
            .values()
            .sum::<u32>(),
        1
    );
    let chest_cells = containers.cells(&chest);
    let player_slots = held(&player.inventory.slots);
    let ender_slots = held(&player.ender);
    assert_eq!(ender_slots, [(0, "minecraft:emerald".to_owned(), 12)]);
    let saved_stats = stats_of(&player);
    let saved_effects = effects_of(&player);

    let states = [
        (
            CHEST,
            vanilla.registries.blocks.parse_state(CHEST_STATE).unwrap(),
        ),
        (
            FURNACE,
            vanilla
                .registries
                .blocks
                .parse_state(FURNACE_STATE)
                .unwrap(),
        ),
    ];
    {
        let mut map = chunk_map(&vanilla, &world);
        map.load_now(ChunkPos::new(0, 0));
        let at = |(x, y, z): (i32, i32, i32)| BlockPos::new(x, y, z);
        let edits: Vec<(BlockPos, BlockStateId)> = states
            .iter()
            .map(|&(pos, state)| (at(pos), state))
            .collect();
        map.set_blocks(&edits);
        let kind = |pos| match pos {
            CHEST => block_entity_kind(&block(CHEST_STATE)),
            FURNACE => block_entity_kind(&block(FURNACE_STATE)),
            _ => None,
        };
        let changes: Vec<(BlockPos, Option<Tag>)> = containers
            .take_block_entities(kind)
            .into_iter()
            .map(|(pos, tag)| (at(pos), tag))
            .collect();
        assert_eq!(changes.len(), 2);
        map.set_block_entities(&changes);
    }
    let save = first.save.take().unwrap();
    save.write(
        &minecraft_survival::save::world_spawn(Dimension::Overworld, SPAWN),
        Some(minecraft_survival::save::WorldClocks {
            overworld: day,
            the_end: minecraft_survival::save::END_CLOCK_START,
        }),
        &rules,
        Difficulty::Hard,
    )
    .unwrap();
    save.write_weather(&weather).unwrap();
    save.write_player(&uuid, &player.data()).unwrap();
    drop(save);
    drop(first);

    check_world_files(&world, &uuid);

    let again = open_world(&saves.0, Some("compat"), None, 77);
    assert!(again.warnings.is_empty(), "{:?}", again.warnings);
    assert_eq!(again.seed, SEED);
    assert_eq!(again.spawn.as_ref().map(|spawn| spawn.pos), Some(SPAWN));
    assert_eq!(again.day_ticks, Some(13_500.0));
    assert_eq!(again.difficulty, Difficulty::Hard);
    assert_eq!(again.weather, Some(weather));
    let back = joined(&again, data, &rules);
    assert_eq!(back.uuid.0, uuid);
    assert_eq!(held(&back.inventory.slots), player_slots);
    assert_eq!(
        back.inventory.slots[0].as_ref().map(|s| &s.components),
        Some(&sword(data).components)
    );
    assert_eq!(back.inventory.slots[1], Some(gun(data)));
    assert_eq!(back.selected, 4);
    assert_eq!(held(&back.ender), ender_slots);
    assert_eq!(back.mode, PlayerMode::new(GameMode::Survival));
    assert_eq!(stats_of(&back), saved_stats);
    assert_eq!(effects_of(&back), saved_effects);
    assert_eq!(back.arrival, player.place);
    assert_eq!(back.respawn, player.respawn);
    assert_eq!((back.portal_cooldown, back.seen_credits), (7, true));
    assert!(!back.dead);

    let mut map = chunk_map(&vanilla, &world);
    let chunk = map.load_now(ChunkPos::new(0, 0));
    for (pos, state) in states {
        assert_eq!(
            chunk.block(pos.0 as usize, pos.1, pos.2 as usize),
            state,
            "{pos:?}"
        );
    }
    let entities = &chunk.block_entities.entities;
    check_block_entities(&vanilla, &entities[&CHEST], &entities[&FURNACE]);
    let chest_gun = list_kinds(entities[&CHEST].get("Items").unwrap(), "Items")
        .iter()
        .find(|item| item.get("Slot") == Some(&Tag::Byte(14)))
        .unwrap();
    check_gun_kinds(chest_gun, "the gun in the chest");
    let mut reloaded = Containers::default();
    reloaded.set_book(data.stations.clone());
    reloaded.load_chunk(&chunk, &data.recipes);
    assert_eq!(reloaded.cells(&chest), chest_cells);
    assert_eq!(chest_cells[14], Some(gun(data)));
    let oven = reloaded.oven(FURNACE).unwrap();
    assert_eq!(oven.data(), cooking);
    assert_eq!(
        oven.recipes_used,
        containers.oven(FURNACE).unwrap().recipes_used
    );
    for _ in 0..400 {
        containers.tick(&data.recipes);
        reloaded.tick(&data.recipes);
    }
    assert_eq!(
        reloaded.block_entity_tag(FURNACE, "minecraft:furnace"),
        containers.block_entity_tag(FURNACE, "minecraft:furnace"),
        "the furnace goes on where it stopped"
    );
    drop(map);
    drop(again);
}
