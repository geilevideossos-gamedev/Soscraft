//! Natural spawning around players each tick (`NaturalSpawner.spawnForChunk`
//! and `spawnCategoryForPosition` with their checks), `DifficultyInstance`,
//! the monster spawn rules (`Monster.checkMonsterSpawnRules`,
//! `Slime.checkSlimeSpawnRules`) and the monsters' `finalizeSpawn` (zombies,
//! zombie villagers, skeletons, spiders, creepers, endermen, witches, slimes
//! and zombie horses), from the pinned 26.3 common JAR.
//!
//! Draws from an entity's own random (unseeded in vanilla: a zombie's
//! reinforcement chance and knockback bonus, a zombie villager's profession)
//! come from a random seeded by the entity's UUID here; vanilla runs differ
//! in them too. Structure spawn overrides (swamp huts, ocean monuments,
//! outposts, fortresses), spawn costs and enchanted spawn equipment are not
//! ported and are noted when reached.

use super::{place, set, set_attribute_base, spawn_aabb, type_info, CreatureSpawns, Creatures, Group, RandomRef, Rules, SpawnLevel, SpawnerData, StructureHere, TypeInfo, Walk};
use minecraftoss_core::block::flags;
use minecraftoss_core::entity_data::finalize_mob;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::{LegacyRandom, RandomSource};
use minecraftoss_core::{BiomeId, BlockPos, ChunkPos};
use std::collections::BTreeMap;

/// `MobCategory` (the spawning categories, in declaration order).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum MobCategory {
    Monster,
    Creature,
    Ambient,
    Axolotls,
    UndergroundWaterCreature,
    WaterCreature,
    WaterAmbient,
}

impl MobCategory {
    pub const COUNT: usize = 7;
    /// `NaturalSpawner.SPAWNING_CATEGORIES`.
    pub const SPAWNING: [Self; Self::COUNT] = [
        Self::Monster,
        Self::Creature,
        Self::Ambient,
        Self::Axolotls,
        Self::UndergroundWaterCreature,
        Self::WaterCreature,
        Self::WaterAmbient,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Monster => "monster",
            Self::Creature => "creature",
            Self::Ambient => "ambient",
            Self::Axolotls => "axolotls",
            Self::UndergroundWaterCreature => "underground_water_creature",
            Self::WaterCreature => "water_creature",
            Self::WaterAmbient => "water_ambient",
        }
    }

    pub fn parse(name: &str) -> Option<Self> {
        Self::SPAWNING.into_iter().find(|c| c.name() == name)
    }

    /// `getMaxInstancesPerChunk`.
    pub fn max_instances_per_chunk(self) -> i32 {
        match self {
            Self::Monster => 70,
            Self::Creature => 10,
            Self::Ambient => 15,
            Self::WaterAmbient => 20,
            Self::Axolotls | Self::UndergroundWaterCreature | Self::WaterCreature => 5,
        }
    }

    /// `isFriendly`.
    pub fn friendly(self) -> bool {
        self != Self::Monster
    }

    /// `isPersistent`: spawned only every 400 ticks.
    pub fn persistent(self) -> bool {
        self == Self::Creature
    }

    /// `getDespawnDistance`.
    pub fn despawn_distance(self) -> i32 {
        if self == Self::WaterAmbient { 64 } else { 128 }
    }

    /// `getNoDespawnDistance`.
    pub fn no_despawn_distance(self) -> i32 {
        32
    }

    /// `EntityType.getCategory` for the mob types spawning names; none for
    /// `MISC` and types not listed.
    pub fn of_type(kind: &str) -> Option<Self> {
        Some(match kind.trim_start_matches("minecraft:") {
            "zombie" | "zombie_villager" | "zombie_horse" | "skeleton" | "creeper" | "spider" | "enderman" | "witch" | "slime" | "husk" | "stray" | "drowned"
            | "bogged" | "parched" | "camel_husk" | "cave_spider" | "silverfish" | "phantom" | "blaze" | "ghast" | "magma_cube" | "piglin" | "hoglin"
            | "zombified_piglin" | "pillager" | "shulker" | "wither_skeleton" | "piglin_brute" | "zoglin" | "vindicator" | "evoker" | "vex" | "ravager" => Self::Monster,
            "cow" | "pig" | "sheep" | "chicken" | "horse" | "donkey" | "mule" | "mooshroom" | "rabbit" | "wolf" | "fox" | "goat" | "llama" | "panda"
            | "polar_bear" | "cat" | "ocelot" | "parrot" | "frog" | "camel" | "armadillo" | "turtle" | "strider" => Self::Creature,
            "bat" => Self::Ambient,
            "axolotl" => Self::Axolotls,
            "glow_squid" => Self::UndergroundWaterCreature,
            "squid" | "dolphin" => Self::WaterCreature,
            "cod" | "salmon" | "tropical_fish" | "pufferfish" => Self::WaterAmbient,
            _ => return None,
        })
    }
}

pub const BLACK_CAT_STRUCTURES: [&str; 1] = ["minecraft:swamp_hut"];

/// `EntityType.canSpawnFarFromPlayer`: creature (and misc) types, pillagers
/// and shulkers.
fn can_spawn_far_from_player(kind: &str) -> bool {
    MobCategory::of_type(kind) == Some(MobCategory::Creature) || matches!(kind, "minecraft:pillager" | "minecraft:shulker")
}

/// `DifficultyInstance`.
#[derive(Clone, Copy, Debug)]
pub struct DifficultyInstance {
    /// `Difficulty.getId`: 0 peaceful to 3 hard.
    pub base: i32,
    pub effective: f32,
}

impl DifficultyInstance {
    /// `DifficultyInstance(base, totalGameTime, localGameTime, moonBrightness)`.
    pub fn new(base: i32, total_game_time: i64, local_game_time: i64, moon_brightness: f32) -> Self {
        if base == 0 {
            return Self { base, effective: 0.0 };
        }
        let hard = base == 3;
        let mut scale = 0.75f32;
        let global = ((total_game_time as f32 + -72000.0) / 1_440_000.0).clamp(0.0, 1.0) * 0.25;
        scale += global;
        let mut local = 0.0f32;
        local += (local_game_time as f32 / 3_600_000.0).clamp(0.0, 1.0) * if hard { 1.0 } else { 0.75 };
        local += (moon_brightness * 0.25).clamp(0.0, global);
        if base == 1 {
            local *= 0.5;
        }
        scale += local;
        Self { base, effective: base as f32 * scale }
    }

    /// `getSpecialMultiplier`.
    pub fn special_multiplier(self) -> f32 {
        if self.effective < 2.0 {
            0.0
        } else if self.effective > 4.0 {
            1.0
        } else {
            (self.effective - 2.0) / 2.0
        }
    }
}

/// `DimensionType.MOON_BRIGHTNESS_PER_PHASE`, by moon phase.
pub const MOON_BRIGHTNESS_PER_PHASE: [f32; 8] = [1.0, 0.75, 0.5, 0.25, 0.0, 0.25, 0.5, 0.75];

/// What spawning reads besides the level's blocks and light.
pub struct SpawnContext<'a> {
    /// The feet of every player that is not a spectator.
    pub players: &'a [[f64; 3]],
    /// The respawn block when it is in this dimension (`getRespawnData`).
    pub respawn: Option<BlockPos>,
    /// `Difficulty.getId`.
    pub difficulty: i32,
    /// `getOverworldClockTime`.
    pub overworld_time: i64,
    /// `getMoonBrightness` (by the moon phase).
    pub moon_brightness: f32,
    /// `Level.isRaining` and `isThundering`.
    pub raining: bool,
    pub thundering: bool,
    /// The world seed (slime chunks).
    pub seed: i64,
    /// `SURFACE_SLIME_SPAWN_CHANCE` in a biome (swamp surfaces).
    pub surface_slime_chance: &'a dyn Fn(BiomeId) -> f32,
    /// `SpecialDates.isHalloween`.
    pub halloween: bool,
    /// `canSpawnEntitiesInChunk`: the chunk's entities tick.
    pub can_spawn_in_chunk: &'a dyn Fn(ChunkPos) -> bool,
    /// Boxes of entities that block building (living entities): they
    /// obstruct spawns (`isUnobstructed`), and spawned mobs join them.
    pub obstacles: Vec<[f64; 6]>,
    /// Boxes of chickens nobody rides (a baby zombie's jockey search).
    pub chickens: Vec<[f64; 6]>,
    pub structures: &'a dyn Fn(BlockPos) -> Vec<StructureHere>,
    pub spawning_monsters: bool,
}

/// `SpawnState`'s tests around one spawn: the spawn potential (`canSpawn`)
/// and the counts after it (`afterSpawn`), given the type's spawn cost at
/// the position (energy budget and charge).
pub trait SpawnCallbacks {
    fn can_spawn(&mut self, _kind: &str, _pos: BlockPos, _cost: Option<(f64, f64)>) -> bool {
        true
    }
    fn after_spawn(&mut self, _kind: &str, _pos: BlockPos, _cost: Option<(f64, f64)>) {}
}

/// No mob cap or potential (`spawnCategoryForPosition`'s debug entry).
pub struct Unlimited;
impl SpawnCallbacks for Unlimited {}

/// `SpawnGroupData` a monster type carries through its group.
#[derive(Clone, Copy, Debug)]
enum MonsterGroup {
    None,
    /// `Zombie.ZombieGroupData`.
    Zombie { baby: bool, can_spawn_jockey: bool },
    /// `Spider.SpiderEffectsGroupData` (no effect below Hard).
    Spider,
    /// `AgeableMobGroupData` (slimes, zombie horses).
    Ageable { should_spawn_baby: bool, chance: f32, size: i32 },
}

fn byte(value: bool) -> Tag {
    Tag::Byte(i8::from(value))
}

pub const PATROL_LEADER_CHANCE: f32 = 0.06;
pub const PATROL_TARGET_SPREAD: i32 = 500;
pub const OMINOUS_BANNER_NAME: &str = "block.minecraft.ominous_banner";
pub const OMINOUS_BANNER_PATTERNS: [(&str, &str); 8] = [
    ("rhombus", "cyan"),
    ("stripe_bottom", "light_gray"),
    ("stripe_center", "gray"),
    ("border", "light_gray"),
    ("stripe_middle", "black"),
    ("half_horizontal", "light_gray"),
    ("circle", "light_gray"),
    ("border", "black"),
];

pub fn ominous_banner() -> Tag {
    let text = |value: &str| Tag::String(value.to_owned());
    let layers = OMINOUS_BANNER_PATTERNS
        .iter()
        .map(|(pattern, color)| Tag::Compound(BTreeMap::from([("color".to_owned(), text(color)), ("pattern".to_owned(), text(&format!("minecraft:{pattern}")))])))
        .collect();
    let hidden = Tag::Compound(BTreeMap::from([("hidden_components".to_owned(), Tag::List(vec![text("minecraft:banner_patterns")]))]));
    let name = Tag::Compound(BTreeMap::from([("translate".to_owned(), text(OMINOUS_BANNER_NAME))]));
    let components = BTreeMap::from([
        ("minecraft:banner_patterns".to_owned(), Tag::List(layers)),
        ("minecraft:tooltip_display".to_owned(), hidden),
        ("minecraft:item_name".to_owned(), name),
        ("minecraft:rarity".to_owned(), text("uncommon")),
    ]);
    Tag::Compound(BTreeMap::from([
        ("components".to_owned(), Tag::Compound(components)),
        ("count".to_owned(), Tag::Int(1)),
        ("id".to_owned(), text("minecraft:white_banner")),
    ]))
}

fn hoglin_spawns_as_baby(random: &mut dyn RandomSource, group: &mut MonsterGroup) -> bool {
    let baby = random.next_f32() < 0.2;
    if matches!(group, MonsterGroup::None) {
        *group = MonsterGroup::Ageable { should_spawn_baby: true, chance: 0.05, size: 0 };
    }
    let MonsterGroup::Ageable { should_spawn_baby, chance, size } = group else { return baby };
    let grouped = *should_spawn_baby && *size > 0 && random.next_f32() <= *chance;
    *size += 1;
    baby || grouped
}

pub(super) struct PiglinSpawn {
    pub(super) baby: bool,
    pub(super) weapon: Option<&'static str>,
    pub(super) hunted: i32,
    pub(super) armor: [Option<(&'static str, &'static str)>; 4],
}

pub(super) fn piglin_spawn(level: &mut dyn RandomSource, own: &mut impl RandomSource) -> PiglinSpawn {
    let baby = level.next_f32() < 0.2;
    let weapon = if baby {
        None
    } else if f64::from(own.next_f32()) < 0.5 {
        Some("minecraft:crossbow")
    } else if own.next_i32_bound(10) == 0 {
        Some("minecraft:golden_spear")
    } else {
        Some("minecraft:golden_sword")
    };
    let hunted = level.next_i32_bound(2400 - 600 + 1) + 600;
    let mut armor = [None; 4];
    if !baby {
        let pieces = [("head", "minecraft:golden_helmet"), ("chest", "minecraft:golden_chestplate"), ("legs", "minecraft:golden_leggings"), ("feet", "minecraft:golden_boots")];
        for (cell, piece) in armor.iter_mut().zip(pieces) {
            if level.next_f32() < 0.1 {
                *cell = Some(piece);
            }
        }
    }
    PiglinSpawn { baby, weapon, hunted, armor }
}

fn drowned_weapon(random: &mut dyn RandomSource) -> Option<&'static str> {
    if f64::from(random.next_f32()) > 0.9 {
        Some(if random.next_i32_bound(16) < 10 { "minecraft:trident" } else { "minecraft:fishing_rod" })
    } else {
        None
    }
}

fn drowned_shell(random: &mut dyn RandomSource) -> bool {
    random.next_f32() < 0.03
}

fn drowned_of(root: &mut Tag) -> Option<&mut Tag> {
    if root.get("id").and_then(Tag::as_str) == Some("minecraft:drowned") {
        return Some(root);
    }
    match root {
        Tag::Compound(map) => match map.get_mut("Passengers") {
            Some(Tag::List(riders)) => riders.first_mut(),
            _ => None,
        },
        _ => None,
    }
}

pub(super) fn piglin_may_stand_on(below: &str) -> bool {
    below != "minecraft:nether_wart_block"
}

/// An equipment slot holding one item (`setItemSlot(slot, new ItemStack(item))`).
pub(super) fn equip(tag: &mut Tag, slot: &str, item: &str) {
    let Tag::Compound(map) = tag else { return };
    let equipment = map.entry("equipment".to_owned()).or_insert_with(|| Tag::Compound(BTreeMap::new()));
    if let Tag::Compound(slots) = equipment {
        let mut stack = BTreeMap::new();
        stack.insert("count".to_owned(), Tag::Int(1));
        stack.insert("id".to_owned(), Tag::String(item.to_owned()));
        slots.insert(slot.to_owned(), Tag::Compound(stack));
    }
}

fn has_equipment(tag: &Tag, slot: &str) -> bool {
    tag.get("equipment").and_then(|e| e.get(slot)).is_some()
}

/// `AttributeMap.getInstance`: the attribute's saved entry, created with
/// the type's default base when first touched.
pub(super) fn attribute<'t>(tag: &'t mut Tag, id: &str, default_base: f64) -> Option<&'t mut BTreeMap<String, Tag>> {
    let Tag::Compound(map) = tag else { return None };
    let Tag::List(list) = map.entry("attributes".to_owned()).or_insert_with(|| Tag::List(Vec::new())) else { return None };
    let index = match list.iter().position(|a| a.get("id").and_then(Tag::as_str) == Some(id)) {
        Some(i) => i,
        None => {
            let mut entry = BTreeMap::new();
            entry.insert("id".to_owned(), Tag::String(id.to_owned()));
            entry.insert("base".to_owned(), Tag::Double(default_base));
            list.push(Tag::Compound(entry));
            list.len() - 1
        }
    };
    match &mut list[index] {
        Tag::Compound(entry) => Some(entry),
        _ => None,
    }
}

/// `addOrReplacePermanentModifier`.
fn add_modifier(tag: &mut Tag, id: &str, default_base: f64, modifier_id: &str, amount: f64, operation: &str) {
    let Some(entry) = attribute(tag, id, default_base) else { return };
    let Tag::List(modifiers) = entry.entry("modifiers".to_owned()).or_insert_with(|| Tag::List(Vec::new())) else { return };
    modifiers.retain(|m| m.get("id").and_then(Tag::as_str) != Some(modifier_id));
    let mut modifier = BTreeMap::new();
    modifier.insert("amount".to_owned(), Tag::Double(amount));
    modifier.insert("id".to_owned(), Tag::String(modifier_id.to_owned()));
    modifier.insert("operation".to_owned(), Tag::String(operation.to_owned()));
    modifiers.push(Tag::Compound(modifier));
}

/// The follow range base a type's attribute supplier gives.
fn follow_range_base(kind: &str) -> f64 {
    match kind {
        "minecraft:zombie" | "minecraft:zombie_villager" | "minecraft:husk" | "minecraft:drowned" | "minecraft:zombified_piglin" => 35.0,
        "minecraft:enderman" => 64.0,
        "minecraft:ghast" => 100.0,
        "minecraft:blaze" => 48.0,
        "minecraft:piglin_brute" => 12.0,
        "minecraft:pillager" | "minecraft:ravager" => 32.0,
        "minecraft:vindicator" | "minecraft:evoker" => 12.0,
        _ => 16.0,
    }
}

/// An entity's own random, seeded from its UUID (vanilla's is unseeded).
fn entity_random(uuid: [i32; 4]) -> LegacyRandom {
    let seed = uuid.iter().fold(0x5DEE_CE66_Du64, |h, &w| h.rotate_left(17) ^ u64::from(w as u32));
    LegacyRandom::new(seed as i64)
}

/// `WorldgenRandom.seedSlimeChunk(x, z, seed, 987234911).nextInt(10) == 0`.
pub fn is_slime_chunk(chunk: ChunkPos, seed: i64) -> bool {
    let (x, z) = (chunk.x, chunk.z);
    let mixed = seed
        .wrapping_add(i64::from(x.wrapping_mul(x).wrapping_mul(4_987_142)))
        .wrapping_add(i64::from(x.wrapping_mul(5_947_611)))
        .wrapping_add(i64::from(z.wrapping_mul(z)).wrapping_mul(4_392_871))
        .wrapping_add(i64::from(z.wrapping_mul(389_711)))
        ^ 987_234_911;
    LegacyRandom::new(mixed).next_i32_bound(10) == 0
}

/// `VillagerType.BY_BIOME` (plains otherwise).
fn villager_type(biome: &str) -> &'static str {
    match biome.trim_start_matches("minecraft:") {
        "badlands" | "desert" | "eroded_badlands" | "wooded_badlands" => "minecraft:desert",
        "bamboo_jungle" | "jungle" | "sparse_jungle" => "minecraft:jungle",
        "savanna_plateau" | "savanna" | "windswept_savanna" => "minecraft:savanna",
        "deep_frozen_ocean" | "frozen_ocean" | "frozen_river" | "ice_spikes" | "snowy_beach" | "snowy_taiga" | "snowy_plains" | "grove" | "snowy_slopes"
        | "frozen_peaks" | "jagged_peaks" => "minecraft:snow",
        "swamp" | "mangrove_swamp" => "minecraft:swamp",
        "old_growth_spruce_taiga" | "old_growth_pine_taiga" | "windswept_gravelly_hills" | "windswept_hills" | "taiga" | "windswept_forest" => "minecraft:taiga",
        _ => "minecraft:plains",
    }
}

/// `VillagerProfession` in registration order.
const PROFESSIONS: [&str; 15] = [
    "none",
    "armorer",
    "butcher",
    "cartographer",
    "cleric",
    "farmer",
    "fisherman",
    "fletcher",
    "leatherworker",
    "librarian",
    "mason",
    "nitwit",
    "shepherd",
    "toolsmith",
    "weaponsmith",
];

/// `Mob.getEquipmentForSlot`: leather, copper, gold, chainmail, iron, diamond.
fn equipment_for_slot(slot: &str, armor_type: i32) -> Option<String> {
    let material = ["leather", "copper", "golden", "chainmail", "iron", "diamond"].get(usize::try_from(armor_type).ok()?)?;
    let piece = match slot {
        "head" => "helmet",
        "chest" => "chestplate",
        "legs" => "leggings",
        "feet" => "boots",
        _ => return None,
    };
    Some(format!("minecraft:{material}_{piece}"))
}

/// The box of a mob after `finalizeSpawn` (babies and slimes resize).
pub(super) fn mob_box(tag: &Tag, info: &TypeInfo, at: [f64; 3]) -> [f64; 6] {
    let (mut width, mut height) = (info.width, info.height);
    if tag.get("IsBaby").and_then(Tag::as_i64) == Some(1) {
        (width, height) = (0.49, 0.98);
    }
    if let Some(size) = tag.get("Size").and_then(Tag::as_i64) {
        let scale = (size + 1) as f32;
        (width, height) = (width * scale, height * scale);
    }
    let half = f64::from(width / 2.0);
    [at[0] - half, at[1], at[2] - half, at[0] + half, at[1] + f64::from(height), at[2] + half]
}

fn overlaps(a: &[f64; 6], b: &[f64; 6]) -> bool {
    a[0] < b[3] && a[3] > b[0] && a[1] < b[4] && a[4] > b[1] && a[2] < b[5] && a[5] > b[2]
}

impl CreatureSpawns {
    /// `MobSpawnSettings.getMobSpawnCost` of the biome at a position.
    pub(super) fn cost_at(&self, level: &dyn SpawnLevel, kind: &str, pos: BlockPos) -> Option<(f64, f64)> {
        self.costs[usize::from(level.biome(pos).0)].get(kind).copied()
    }

    pub fn kinds_at(&self, level: &dyn SpawnLevel, structures: &dyn Fn(BlockPos) -> Vec<StructureHere>, category: MobCategory, pos: BlockPos) -> Vec<String> {
        self.mobs_at(level, structures, category, pos).entries.iter().map(|entry| entry.kind.clone()).collect()
    }

    fn mobs_at(&self, level: &dyn SpawnLevel, structures: &dyn Fn(BlockPos) -> Vec<StructureHere>, category: MobCategory, pos: BlockPos) -> std::borrow::Cow<'_, Creatures> {
        let here = structures(pos);
        if category == MobCategory::Monster && here.iter().any(|s| s.name == "minecraft:fortress" && s.in_box) {
            let below = self.registries.blocks.block(self.registries.blocks.block_of(level.block(pos.below()))).name.as_str() == "minecraft:nether_bricks";
            if below {
                return std::borrow::Cow::Borrowed(&self.fortress_enemies);
            }
        }
        for structure in &here {
            let spawns = self.structure_spawns.iter().find(|(name, _)| *name == structure.name).and_then(|(_, lists)| lists[category as usize].as_ref());
            if let Some((piece, list)) = spawns
                && ((*piece && structure.in_piece) || (!*piece && structure.in_box))
            {
                return std::borrow::Cow::Borrowed(list);
            }
        }
        let biome = level.biome(pos);
        std::borrow::Cow::Borrowed(&self.by_category[usize::from(biome.0)][category as usize])
    }

    /// `NaturalSpawner.spawnCategoryForPosition`: up to three groups
    /// around `start`, each a random walk that spawns the first type drawn
    /// where it may. `chunk` is the chunk spawning (the start's chunk).
    pub fn spawn_category_for_position(
        &self,
        level: &mut dyn SpawnLevel,
        context: &mut SpawnContext,
        callbacks: &mut dyn SpawnCallbacks,
        category: MobCategory,
        chunk: ChunkPos,
        start: BlockPos,
    ) {
        let y_start = start.y;
        if self.registries.blocks.is(level.block(start), flags::REDSTONE_CONDUCTOR) {
            return;
        }
        let mut cluster_size = 0;
        for _ in 0..3 {
            let (mut x, mut z) = (start.x, start.z);
            let mut current: Option<SpawnerData> = None;
            let mut monster_group = MonsterGroup::None;
            let mut animal_group = Group::None;
            let mut max = (level.random().next_f32() * 4.0).ceil() as i32;
            let mut ll = 0;
            while ll < max {
                ll += 1;
                x += level.random().next_i32_bound(6) - level.random().next_i32_bound(6);
                z += level.random().next_i32_bound(6) - level.random().next_i32_bound(6);
                let pos = BlockPos::new(x, y_start, z);
                let (xx, zz) = (f64::from(x) + 0.5, f64::from(z) + 0.5);
                // `getNearestPlayer(xx, yStart, zz, -1, false)`.
                let Some(nearest) = context
                    .players
                    .iter()
                    .map(|p| {
                        let (dx, dy, dz) = (p[0] - xx, p[1] - f64::from(y_start), p[2] - zz);
                        dx * dx + dy * dy + dz * dz
                    })
                    .min_by(f64::total_cmp)
                else {
                    continue;
                };
                if !Self::right_distance(context, chunk, pos, nearest) {
                    continue;
                }
                if current.is_none() {
                    let Some(data) = self.random_spawn_mob_at(level, context, category, pos) else { break };
                    // `count.sample`: a constant draws nothing.
                    max = if data.constant { data.min } else { level.random().next_i32_bound(data.max - data.min + 1) + data.min };
                    current = Some(data);
                }
                let data = current.clone().expect("drawn above");
                let Some(info) = type_info(&data.kind) else {
                    level.note_unsupported(&format!("natural spawning of {}", data.kind));
                    continue;
                };
                if !self.valid_spawn_position_for_type(level, context, &data, &info, pos, nearest) {
                    continue;
                }
                let cost = self.cost_at(level, &data.kind, pos);
                if !callbacks.can_spawn(&data.kind, pos, cost) {
                    continue;
                }
                // `getMobForSpawn`: types not allowed in peaceful are not created.
                if context.difficulty == 0 && MobCategory::of_type(&data.kind) == Some(MobCategory::Monster) {
                    return;
                }
                let Some(mut tag) = self.registries.entities.as_ref().and_then(|c| c.default_tag(&data.kind)) else {
                    level.note_unsupported(&format!("creating {}", data.kind));
                    return;
                };
                let y_rot = level.random().next_f32() * 360.0;
                let uuid = level.next_uuid();
                let at = [xx, f64::from(y_start), zz];
                place(&mut tag, at, y_rot, 0.0, uuid);
                if !self.valid_position_for_mob(level, context, &data.kind, &info, pos, at, nearest) {
                    continue;
                }
                let difficulty = Self::difficulty_at(level, context, pos);
                let root = if info.rules == Rules::Strider {
                    self.finalize_strider(level, context, tag, pos, difficulty)
                } else if info.rules.creature() {
                    self.finalize_spawn(level, &data.kind, &mut tag, pos, &mut animal_group);
                    tag
                } else {
                    self.finalize_monster(level, context, &data.kind, tag, pos, difficulty, &mut monster_group, true)
                };
                cluster_size += 1;
                let riding = root.get("Passengers").is_some();
                for (entity, kind) in Self::self_and_passengers(&root) {
                    let entity_info = type_info(kind).unwrap_or(info);
                    let bb = mob_box(entity, &entity_info, at);
                    context.obstacles.push(bb);
                    if kind == "minecraft:chicken" && !riding {
                        context.chickens.push(bb);
                    }
                }
                level.add_entity(root);
                callbacks.after_spawn(&data.kind, pos, cost);
                if cluster_size >= info.cluster {
                    return;
                }
            }
        }
    }

    /// An entity tag and its passengers' tags, with their types.
    fn self_and_passengers(tag: &Tag) -> Vec<(&Tag, &str)> {
        let mut out = vec![(tag, tag.get("id").and_then(Tag::as_str).unwrap_or(""))];
        for passenger in tag.get("Passengers").and_then(Tag::as_list).into_iter().flatten() {
            out.extend(Self::self_and_passengers(passenger));
        }
        out
    }

    /// `isRightDistanceToPlayerAndSpawnPoint`.
    fn right_distance(context: &SpawnContext, chunk: ChunkPos, pos: BlockPos, nearest: f64) -> bool {
        if nearest <= 576.0 {
            return false;
        }
        if let Some(r) = context.respawn {
            // `closerToCenterThan`: from the respawn block's centre.
            let (dx, dy, dz) = (
                f64::from(r.x) + 0.5 - (f64::from(pos.x) + 0.5),
                f64::from(r.y) + 0.5 - f64::from(pos.y),
                f64::from(r.z) + 0.5 - (f64::from(pos.z) + 0.5),
            );
            if dx * dx + dy * dy + dz * dz < 24.0 * 24.0 {
                return false;
            }
        }
        let at = pos.chunk();
        at == chunk || (context.can_spawn_in_chunk)(at)
    }

    /// `getRandomSpawnMobAt`.
    fn random_spawn_mob_at(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, category: MobCategory, pos: BlockPos) -> Option<SpawnerData> {
        let biome = level.biome(pos);
        if category == MobCategory::WaterAmbient
            && self.reduced_water_ambient.is_some_and(|t| self.registries.biome_in_tag(biome, t))
            && level.random().next_f32() < 0.98
        {
            return None;
        }
        let list = self.mobs_at(level, context.structures, category, pos).into_owned();
        list.pick(level.random()).cloned()
    }

    /// `isValidSpawnPostitionForType`.
    fn valid_spawn_position_for_type(
        &self,
        level: &mut dyn SpawnLevel,
        context: &SpawnContext,
        data: &SpawnerData,
        info: &TypeInfo,
        pos: BlockPos,
        nearest: f64,
    ) -> bool {
        let Some(type_category) = MobCategory::of_type(&data.kind) else { return false };
        let despawn = f64::from(type_category.despawn_distance());
        if !can_spawn_far_from_player(&data.kind) && nearest > despawn * despawn {
            return false;
        }
        // `canSpawnMobAt`: the list here holds the same `SpawnerData`.
        let here = self.mobs_at(level, context.structures, type_category, pos);
        if !here.entries.iter().any(|e| e.kind == data.kind && e.min == data.min && e.max == data.max && e.constant == data.constant) {
            return false;
        }
        if !self.is_spawn_position_ok(level, &data.kind, info, pos) {
            return false;
        }
        if !self.natural_spawn_rules(level, context, &data.kind, info, pos) {
            return false;
        }
        self.no_collision(level, spawn_aabb(info, f64::from(pos.x) + 0.5, f64::from(pos.y), f64::from(pos.z) + 0.5))
    }

    pub fn spawner_may_spawn(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, kind: &str, at: [f64; 3]) -> bool {
        let Some(info) = type_info(kind) else { return true };
        let pos = BlockPos::new(at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
        let bb = spawn_aabb(&info, at[0], at[1], at[2]);
        if !self.no_collision(level, bb) {
            return false;
        }
        if context.difficulty == 0 && MobCategory::of_type(kind) == Some(MobCategory::Monster) {
            return false;
        }
        let rules = match info.rules {
            Rules::Monster => self.dark_enough(level, context, pos),
            Rules::SurfaceMonster => self.dark_enough(level, context, pos) && level.sky_brightness(pos) >= 15,
            Rules::Ghast => level.random().next_i32_bound(20) == 0,
            Rules::AnyLightMonster => true,
            Rules::Drowned => self.dark_enough(level, context, pos),
            _ => self.natural_spawn_rules(level, context, kind, &info, pos),
        };
        if !rules {
            return false;
        }
        let walk = match info.walk {
            Walk::Animal => self.mob_check_spawn_rules(level, &info, pos),
            Walk::Monster => -self.pathfinding_cost_from_light(level, pos) >= 0.0,
            Walk::Neutral => true,
        };
        walk && self.check_spawn_obstruction(level, &info, at[0], at[1], at[2]) && !context.obstacles.iter().any(|o| overlaps(&bb, o))
    }

    /// `SpawnPlacements.checkSpawnRules` with reason `NATURAL`.
    fn natural_spawn_rules(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, kind: &str, info: &TypeInfo, pos: BlockPos) -> bool {
        match info.rules {
            Rules::Animal | Rules::SpawnableOn(_) => self.check_spawn_rules(level, info, pos),
            Rules::Monster => self.dark_enough(level, context, pos) && self.mob_spawn_rules(level, kind, info, pos),
            // `canSeeSky`: full sky light, tested after the monster rules.
            Rules::SurfaceMonster => self.dark_enough(level, context, pos) && self.mob_spawn_rules(level, kind, info, pos) && level.sky_brightness(pos) >= 15,
            Rules::Stray => {
                // The sky is seen from the top of any powder snow above.
                let mut sky = pos.above();
                while self.registries.blocks.block(self.registries.blocks.block_of(level.block(sky))).name.as_str() == "minecraft:powder_snow" {
                    sky = sky.above();
                }
                self.dark_enough(level, context, pos) && self.mob_spawn_rules(level, kind, info, pos) && level.sky_brightness(sky.below()) >= 15
            }
            Rules::Slime => self.slime_spawn_rules(level, context, kind, info, pos),
            Rules::MagmaCube => context.difficulty != 0,
            Rules::ZombifiedPiglin => {
                let below = self.registries.blocks.block(self.registries.blocks.block_of(level.block(pos.below()))).name.as_str() == "minecraft:nether_wart_block";
                context.difficulty != 0 && !below
            }
            Rules::Bat => self.bat_spawn_rules(level, kind, info, pos),
            Rules::Ghast => context.difficulty != 0 && level.random().next_i32_bound(20) == 0 && self.mob_spawn_rules(level, kind, info, pos),
            Rules::AnyLightMonster => context.difficulty != 0 && self.mob_spawn_rules(level, kind, info, pos),
            // `pos.y <= seaLevel - 33`, unlit, in water: no random draws.
            Rules::GlowSquid => {
                let state = level.block(pos);
                let water = self.registries.blocks.block(self.registries.blocks.block_of(state)).name.as_str() == "minecraft:water";
                pos.y <= level.sea_level() - 33 && level.raw_brightness(pos, 0) == 0 && water
            }
            Rules::Strider => self.strider_spawn_rules(level, pos),
            Rules::Drowned => self.drowned_spawn_rules(level, context, pos),
            Rules::Hoglin => self.registries.blocks.block(self.registries.blocks.block_of(level.block(pos.below()))).name.as_str() != "minecraft:nether_wart_block",
            Rules::Piglin => piglin_may_stand_on(self.registries.blocks.block(self.registries.blocks.block_of(level.block(pos.below()))).name.as_str()),
            Rules::Ocelot => level.random().next_i32_bound(3) != 0,
            Rules::Unrestricted => true,
            Rules::Patrolling => level.block_brightness(pos) <= 8 && context.difficulty != 0 && self.mob_spawn_rules(level, kind, info, pos),
            Rules::Other => {
                level.note_unsupported(&format!("spawn rules of {kind}"));
                false
            }
        }
    }

    /// `Monster.isDarkEnoughToSpawn`.
    fn dark_enough(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, pos: BlockPos) -> bool {
        let sky = level.sky_brightness(pos);
        if sky > level.random().next_i32_bound(32) {
            return false;
        }
        if self.monster_block_light_limit < 15 && level.block_brightness(pos) > self.monster_block_light_limit {
            return false;
        }
        let darkening = if context.thundering { 10 } else { level.sky_darken() };
        let brightness = level.raw_brightness(pos, darkening);
        brightness <= level.random().next_i32_bound(self.monster_light_test_max + 1)
    }

    fn drowned_spawn_rules(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, pos: BlockPos) -> bool {
        let water = |level: &mut dyn SpawnLevel, at: BlockPos| self.registries.blocks.state(level.block(at)).fluid.is_some_and(|f| f.kind == minecraftoss_core::block::FluidKind::Water);
        if !water(level, pos.below()) {
            return false;
        }
        let biome = level.biome(pos);
        let can_spawn = context.difficulty != 0 && self.dark_enough(level, context, pos) && water(level, pos);
        if self.more_frequent_drowned.is_some_and(|t| self.registries.biome_in_tag(biome, t)) {
            return level.random().next_i32_bound(15) == 0 && can_spawn;
        }
        level.random().next_i32_bound(40) == 0 && pos.y < level.sea_level() - 5 && can_spawn
    }

    /// `Bat.checkBatSpawnRules`.
    fn bat_spawn_rules(&self, level: &mut dyn SpawnLevel, kind: &str, info: &TypeInfo, pos: BlockPos) -> bool {
        if pos.y >= level.height(minecraftoss_core::chunk::HeightmapKind::WorldSurface, pos.x, pos.z) {
            return false;
        }
        if level.random().next_bool() {
            return false;
        }
        let darkening = level.sky_darken();
        if level.raw_brightness(pos, darkening) > level.random().next_i32_bound(4) {
            return false;
        }
        let below = level.block(pos.below());
        if !self.bats_spawnable_on.is_some_and(|t| self.registries.block_in_tag(below, t)) {
            return false;
        }
        self.mob_spawn_rules(level, kind, info, pos)
    }

    /// `Mob.checkMobSpawnRules`: a valid spawn block below.
    fn mob_spawn_rules(&self, level: &mut dyn SpawnLevel, kind: &str, info: &TypeInfo, pos: BlockPos) -> bool {
        let below = level.block(pos.below());
        self.is_valid_spawn(&self.registries, below, kind, info)
    }

    /// `Slime.checkSlimeSpawnRules`: swamp surfaces by the moon, or slime
    /// chunks below 40.
    fn slime_spawn_rules(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, kind: &str, info: &TypeInfo, pos: BlockPos) -> bool {
        if context.difficulty == 0 {
            return false;
        }
        let biome = level.biome(pos);
        let surface = self.surface_slimes.is_some_and(|t| self.registries.biome_in_tag(biome, t));
        if surface && pos.y > 50 && pos.y < 70 {
            let chance = (context.surface_slime_chance)(biome);
            if level.random().next_f32() < chance {
                let darkening = level.sky_darken();
                let brightness = level.raw_brightness(pos, darkening);
                if brightness <= level.random().next_i32_bound(8) {
                    return self.mob_spawn_rules(level, kind, info, pos);
                }
            }
        }
        let slime_chunk = is_slime_chunk(pos.chunk(), context.seed);
        if level.random().next_i32_bound(10) == 0 && slime_chunk && pos.y < 40 {
            return self.mob_spawn_rules(level, kind, info, pos);
        }
        false
    }

    /// `isValidPositionForMob`: close enough to stay, the mob's own spawn
    /// rules (its walk target) and nothing in the way.
    #[allow(clippy::too_many_arguments)]
    fn valid_position_for_mob(
        &self,
        level: &mut dyn SpawnLevel,
        context: &SpawnContext,
        kind: &str,
        info: &TypeInfo,
        block: BlockPos,
        at: [f64; 3],
        nearest: f64,
    ) -> bool {
        let despawn = f64::from(MobCategory::of_type(kind).map_or(128, MobCategory::despawn_distance));
        if nearest > despawn * despawn && info.remove_far {
            return false;
        }
        // `PathfinderMob.checkSpawnRules`: a walk target value of at least 0.
        let walk_target_ok = match info.walk {
            Walk::Animal => self.mob_check_spawn_rules(level, info, block),
            Walk::Monster => -self.pathfinding_cost_from_light(level, block) >= 0.0,
            Walk::Neutral => true,
        };
        if !walk_target_ok || !self.check_spawn_obstruction(level, info, at[0], at[1], at[2]) {
            return false;
        }
        // `isUnobstructed`: no entity that blocks building overlaps.
        let bb = spawn_aabb(info, at[0], at[1], at[2]);
        !context.obstacles.iter().any(|o| overlaps(&bb, o))
    }

    /// `SummonCommand.createEntity`'s mob: the type's fresh tag at `at` (the
    /// new entity's yaw, and a UUID from the level's entity random) with the
    /// command's NBT loaded over it, keeping the command's position; or,
    /// without NBT, `finalizeSpawn(COMMAND)` as the first of its group.
    #[allow(clippy::too_many_arguments)]
    pub fn summon(&self, level: &mut dyn SpawnLevel, context: &mut SpawnContext, kind: &str, at: [f64; 3], nbt: Option<&Tag>, y_rot: f32) -> Result<Tag, String> {
        let Some(mut tag) = self.registries.entities.as_ref().and_then(|c| c.default_tag(kind)) else {
            return Err(format!("Unable to summon {kind}"));
        };
        let uuid = level.next_uuid();
        place(&mut tag, at, y_rot, 0.0, uuid);
        // `Mob.readAdditionalSaveData`: the command's tag is loaded, which
        // reads an absent `CanPickUpLoot` as false (though a villager's
        // constructor set it).
        if let Tag::Compound(map) = &mut tag {
            if map.contains_key("CanPickUpLoot") {
                let piglin = matches!(kind, "minecraft:piglin" | "minecraft:piglin_brute");
                map.insert("CanPickUpLoot".to_owned(), Tag::Byte(i8::from(piglin)));
            }
        }
        if let Some(Tag::Compound(nbt)) = nbt {
            if let Tag::Compound(map) = &mut tag {
                for (key, value) in nbt {
                    if key != "id" && key != "Pos" {
                        map.insert(key.clone(), value.clone());
                    }
                }
            }
            return Ok(tag);
        }
        let pos = BlockPos::new(at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
        if kind == "minecraft:cat" {
            let black = (context.structures)(pos).iter().any(|s| s.in_piece && BLACK_CAT_STRUCTURES.contains(&s.name.as_str()));
            self.finalize_cat(level, &mut tag, black, context.moon_brightness);
            return Ok(tag);
        }
        match type_info(kind) {
            Some(info) if info.rules == Rules::Strider => {
                let difficulty = Self::difficulty_at(level, context, pos);
                Ok(self.finalize_strider(level, context, tag, pos, difficulty))
            }
            Some(info) if info.rules.creature() => {
                self.finalize_spawn(level, kind, &mut tag, pos, &mut Group::None);
                Ok(tag)
            }
            Some(_) => {
                let difficulty = Self::difficulty_at(level, context, pos);
                Ok(self.finalize_monster(level, context, kind, tag, pos, difficulty, &mut MonsterGroup::None, false))
            }
            // Types without a spawn port come as their fresh tag.
            None => Ok(tag),
        }
    }

    /// `ServerLevel.getCurrentDifficultyAt`.
    fn difficulty_at(level: &dyn SpawnLevel, context: &SpawnContext, pos: BlockPos) -> DifficultyInstance {
        let (local, moon) = match level.inhabited_time(pos.chunk()) {
            Some(time) => (time, context.moon_brightness),
            None => (0, 0.0),
        };
        DifficultyInstance::new(context.difficulty, context.overworld_time, local, moon)
    }

    /// `Mob.populateDefaultEquipmentSlots`: an armor set, most likely
    /// leather, one piece after another until a partial-set roll stops it.
    fn populate_armor(level: &mut dyn SpawnLevel, tag: &mut Tag, difficulty: DifficultyInstance) {
        if level.random().next_f32() < 0.15 * difficulty.special_multiplier() {
            let mut armor_type = level.random().next_i32_bound(3);
            for _ in 1..=3 {
                if level.random().next_f32() < 0.1087 {
                    armor_type += 1;
                }
            }
            let partial = if difficulty.base == 3 { 0.1 } else { 0.25 };
            let mut first = true;
            // `EQUIPMENT_POPULATION_ORDER`.
            for slot in ["head", "chest", "legs", "feet"] {
                if !first && level.random().next_f32() < partial {
                    break;
                }
                first = false;
                if !has_equipment(tag, slot) {
                    if let Some(item) = equipment_for_slot(slot, armor_type) {
                        equip(tag, slot, &item);
                    }
                }
            }
        }
    }

    /// `Mob.populateDefaultEquipmentEnchantments`: a roll per equipped item
    /// (the main hand at 0.25, armor at 0.5, times the special multiplier)
    /// in `EquipmentSlot.VALUES` order.
    fn roll_enchantments(level: &mut dyn SpawnLevel, tag: &Tag, difficulty: DifficultyInstance) {
        let special = difficulty.special_multiplier();
        for (slot, chance) in [("mainhand", 0.25f32), ("feet", 0.5), ("legs", 0.5), ("chest", 0.5), ("head", 0.5)] {
            if has_equipment(tag, slot) && level.random().next_f32() < chance * special {
                level.note_unsupported("enchanted spawn equipment");
            }
        }
    }

    /// The Halloween pumpkin head of zombies and skeletons.
    fn halloween_head(level: &mut dyn SpawnLevel, context: &SpawnContext, tag: &mut Tag) {
        if !has_equipment(tag, "head") && context.halloween && level.random().next_f32() < 0.25 {
            let item = if level.random().next_f32() < 0.1 { "minecraft:jack_o_lantern" } else { "minecraft:carved_pumpkin" };
            equip(tag, "head", item);
            level.note_unsupported("pumpkin head drop chance");
        }
    }

    fn finalize_pillager(level: &mut dyn SpawnLevel, tag: &mut Tag, difficulty: DifficultyInstance, rolls_leader: bool, patrol: bool) {
        equip(tag, "mainhand", "minecraft:crossbow");
        Self::roll_enchantments(level, tag, difficulty);
        if level.random().next_i32_bound(300) == 0 {
            let piercing = Tag::Compound(BTreeMap::from([("minecraft:piercing".to_owned(), Tag::Int(1))]));
            let components = Tag::Compound(BTreeMap::from([("minecraft:enchantments".to_owned(), piercing)]));
            if let Tag::Compound(map) = tag
                && let Some(Tag::Compound(slots)) = map.get_mut("equipment")
                && let Some(Tag::Compound(hand)) = slots.get_mut("mainhand")
            {
                hand.insert("components".to_owned(), components);
            }
        }
        Self::finalize_raider(level, tag, rolls_leader, patrol);
        finalize_mob(tag, &mut RandomRef(level.random()));
    }

    fn finalize_raider(level: &mut dyn SpawnLevel, tag: &mut Tag, rolls_leader: bool, patrol: bool) {
        set(tag, "CanJoinRaid", byte(true));
        if rolls_leader && level.random().next_f32() < PATROL_LEADER_CHANCE {
            set(tag, "PatrolLeader", byte(true));
        }
        if tag.get("PatrolLeader").and_then(Tag::as_i64).is_some_and(|leader| leader != 0) {
            if let Tag::Compound(map) = tag {
                let slots = map.entry("equipment".to_owned()).or_insert_with(|| Tag::Compound(BTreeMap::new()));
                if let Tag::Compound(slots) = slots {
                    slots.insert("head".to_owned(), ominous_banner());
                }
                let chances = map.entry("drop_chances".to_owned()).or_insert_with(|| Tag::Compound(BTreeMap::new()));
                if let Tag::Compound(chances) = chances {
                    chances.insert("head".to_owned(), Tag::Float(2.0));
                }
            }
        }
        if patrol {
            set(tag, "Patrolling", byte(true));
        }
    }

    fn finalize_vindicator(level: &mut dyn SpawnLevel, tag: &mut Tag, difficulty: DifficultyInstance, rolls_leader: bool) {
        Self::finalize_raider(level, tag, rolls_leader, false);
        finalize_mob(tag, &mut RandomRef(level.random()));
        equip(tag, "mainhand", "minecraft:iron_axe");
        Self::roll_enchantments(level, tag, difficulty);
    }

    pub fn spawn_patrol(&self, level: &mut dyn SpawnLevel, context: &mut SpawnContext, start: BlockPos) -> Vec<Tag> {
        let size = f64::from(Self::difficulty_at(level, context, start).effective).ceil() as i32 + 1;
        let mut pos = start;
        let mut members = Vec::new();
        for index in 0..size {
            pos.y = level.height(minecraftoss_core::chunk::HeightmapKind::MotionBlockingNoLeaves, pos.x, pos.z);
            match self.patrol_member(level, context, pos, index == 0) {
                Some(member) => members.push(member),
                None if index == 0 => break,
                None => {}
            }
            pos.x += level.random().next_i32_bound(5) - level.random().next_i32_bound(5);
            pos.z += level.random().next_i32_bound(5) - level.random().next_i32_bound(5);
        }
        members
    }

    fn patrol_member(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, pos: BlockPos, leader: bool) -> Option<Tag> {
        let kind = "minecraft:pillager";
        let info = type_info(kind)?;
        if !self.is_valid_empty_spawn_block(&self.registries, level.block(pos), kind, &info) {
            return None;
        }
        if level.block_brightness(pos) > 8 || context.difficulty == 0 || !self.mob_spawn_rules(level, kind, &info, pos) {
            return None;
        }
        let mut tag = self.registries.entities.as_ref()?.default_tag(kind)?;
        let uuid = level.next_uuid();
        if leader {
            let mut own = entity_random(uuid);
            let x = -PATROL_TARGET_SPREAD + own.next_i32_bound(PATROL_TARGET_SPREAD * 2);
            let z = -PATROL_TARGET_SPREAD + own.next_i32_bound(PATROL_TARGET_SPREAD * 2);
            set(&mut tag, "PatrolLeader", byte(true));
            set(&mut tag, "Patrolling", byte(true));
            set(&mut tag, "patrol_target", Tag::IntArray(vec![x, 0, z]));
        }
        place(&mut tag, [f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)], 0.0, 0.0, uuid);
        let _ = attribute(&mut tag, "minecraft:follow_range", follow_range_base(kind));
        let difficulty = Self::difficulty_at(level, context, pos);
        Self::finalize_pillager(level, &mut tag, difficulty, false, true);
        Some(tag)
    }

    /// The monsters' `finalizeSpawn` (with `Mob.finalizeSpawn`'s common
    /// part), returning the entity to add: the mob, or the chicken it rides.
    #[allow(clippy::too_many_arguments)]
    fn finalize_monster(
        &self,
        level: &mut dyn SpawnLevel,
        context: &SpawnContext,
        kind: &str,
        mut tag: Tag,
        pos: BlockPos,
        difficulty: DifficultyInstance,
        group: &mut MonsterGroup,
        natural: bool,
    ) -> Tag {
        let special = difficulty.special_multiplier();
        let uuid = Self::uuid(&tag);
        let mut own = entity_random(uuid);
        // Created before `Mob.finalizeSpawn` adds its bonus.
        let _ = attribute(&mut tag, "minecraft:follow_range", follow_range_base(kind));
        match kind {
            "minecraft:husk" => {
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                let root = self.finalize_zombie(level, context, tag, pos, difficulty, group, &mut own);
                return self.finalize_husk(level, context, root, pos, difficulty, natural);
            }
            "minecraft:zombie" | "minecraft:zombie_villager" | "minecraft:zombified_piglin" => {
                if kind == "minecraft:zombie_villager" {
                    // The constructor's profession, then `finalizeVillagerType`.
                    let profession = PROFESSIONS[own.next_i32_bound(PROFESSIONS.len() as i32) as usize];
                    let biome = self.biome_name(level.biome(pos));
                    let mut data = BTreeMap::new();
                    data.insert("level".to_owned(), Tag::Int(1));
                    data.insert("profession".to_owned(), Tag::String(format!("minecraft:{profession}")));
                    data.insert("type".to_owned(), Tag::String(villager_type(&biome).to_owned()));
                    set(&mut tag, "VillagerData", Tag::Compound(data));
                    set(&mut tag, "VillagerDataFinalized", byte(true));
                }
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                return self.finalize_zombie(level, context, tag, pos, difficulty, group, &mut own);
            }
            "minecraft:drowned" => {
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                let root = self.finalize_zombie(level, context, tag, pos, difficulty, group, &mut own);
                return self.finalize_drowned(level, root, pos, natural);
            }
            // `AbstractSkeleton.finalizeSpawn`, which strays, bogged and
            // parched do not override.
            "minecraft:skeleton" | "minecraft:stray" | "minecraft:bogged" | "minecraft:parched" | "minecraft:wither_skeleton" => {
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                self.finalize_skeleton(level, context, &mut tag, difficulty);
            }
            "minecraft:spider" => {
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                if level.random().next_i32_bound(100) == 0 {
                    // A skeleton jockey: the skeleton finalizes as a spawn of its own.
                    if let Some(mut skeleton) = self.registries.entities.as_ref().and_then(|c| c.default_tag("minecraft:skeleton")) {
                        let skeleton_uuid = level.next_uuid();
                        place(&mut skeleton, Self::position(&tag), Self::y_rot(&tag), 0.0, skeleton_uuid);
                        let _ = attribute(&mut skeleton, "minecraft:follow_range", 16.0);
                        finalize_mob(&mut skeleton, &mut RandomRef(level.random()));
                        self.finalize_skeleton(level, context, &mut skeleton, difficulty);
                        set(&mut tag, "Passengers", Tag::List(vec![skeleton]));
                    }
                }
                if matches!(group, MonsterGroup::None) {
                    *group = MonsterGroup::Spider;
                    if difficulty.base == 3 && level.random().next_f32() < 0.1 * special {
                        level.note_unsupported("spider spawn effect");
                    }
                }
            }
            "minecraft:cave_spider" => {}
            "minecraft:pillager" => Self::finalize_pillager(level, &mut tag, difficulty, true, false),
            "minecraft:vindicator" => Self::finalize_vindicator(level, &mut tag, difficulty, true),
            "minecraft:evoker" => {
                Self::finalize_raider(level, &mut tag, true, false);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:ravager" => {
                set(&mut tag, "CanJoinRaid", byte(true));
                let _ = level.random().next_f32();
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:vex" => {
                equip(&mut tag, "mainhand", "minecraft:iron_sword");
                if let Tag::Compound(map) = &mut tag {
                    let chances = map.entry("drop_chances".to_owned()).or_insert_with(|| Tag::Compound(BTreeMap::new()));
                    if let Tag::Compound(chances) = chances {
                        chances.insert("mainhand".to_owned(), Tag::Float(0.0));
                    }
                }
                Self::roll_enchantments(level, &mut tag, difficulty);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:witch" => {
                // `Raider.finalizeSpawn`, then `PatrollingMonster`'s leader
                // roll (witches never lead), then `Mob`'s part.
                set(&mut tag, "CanJoinRaid", byte(false));
                level.random().next_f32();
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:slime" | "minecraft:magma_cube" => {
                // `AgeableMob.finalizeSpawn` without babies, `Mob`'s part,
                // then `AbstractCubeMob.setSpawnSize`.
                if matches!(group, MonsterGroup::None) {
                    *group = MonsterGroup::Ageable { should_spawn_baby: false, chance: 0.05, size: 0 };
                }
                Self::ageable_monster(level, &mut tag, group);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                let mut scale = level.random().next_i32_bound(3);
                if scale < 2 && level.random().next_f32() < 0.5 * special {
                    scale += 1;
                }
                Self::set_cube_size(&mut tag, 1 << scale);
                if kind == "minecraft:magma_cube" {
                    set_attribute_base(&mut tag, "minecraft:armor", f64::from((1 << scale) * 3));
                }
                // A size other than the default 1 changes the synced size:
                // `onSyncedDataUpdated` turns the cube to its head yaw, the
                // constructor's roll from its own unseeded random.
                if scale > 0 {
                    let yaw = own.next_f32() * std::f64::consts::TAU as f32;
                    set(&mut tag, "Rotation", Tag::List(vec![Tag::Float(yaw), Tag::Float(0.0)]));
                }
            }
            "minecraft:zombie_horse" => {
                // A zombie rider with an iron spear, finalized first.
                if let Some(mut zombie) = self.registries.entities.as_ref().and_then(|c| c.default_tag("minecraft:zombie")) {
                    let zombie_uuid = level.next_uuid();
                    place(&mut zombie, Self::position(&tag), Self::y_rot(&tag), 0.0, zombie_uuid);
                    let mut rider_random = entity_random(zombie_uuid);
                    let _ = attribute(&mut zombie, "minecraft:follow_range", 35.0);
                    finalize_mob(&mut zombie, &mut RandomRef(level.random()));
                    let mut rider_group = MonsterGroup::None;
                    let mut zombie = self.finalize_zombie(level, context, zombie, pos, difficulty, &mut rider_group, &mut rider_random);
                    if zombie.get("id").and_then(Tag::as_str) == Some("minecraft:chicken") {
                        level.note_unsupported("chicken jockey on a zombie horse");
                    }
                    equip(&mut zombie, "mainhand", "minecraft:iron_spear");
                    set(&mut tag, "Passengers", Tag::List(vec![zombie]));
                }
                // `AbstractHorse.finalizeSpawn`: the horse's attributes, then
                // `AgeableMob` and `Mob`.
                if matches!(group, MonsterGroup::None) {
                    *group = MonsterGroup::Ageable { should_spawn_baby: true, chance: 0.2, size: 0 };
                }
                let random = level.random();
                let third = 0.066_666_666_666_666_67;
                let jump = 0.5 + random.next_f64() * third + random.next_f64() * third + random.next_f64() * third;
                let speed = (9.0 + random.next_f64() * 1.0 + random.next_f64() * 1.0 + random.next_f64() * 1.0) / f64::from(42.16f32);
                set_attribute_base(&mut tag, "minecraft:jump_strength", jump);
                set_attribute_base(&mut tag, "minecraft:movement_speed", speed);
                Self::ageable_monster(level, &mut tag, group);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:hoglin" => {
                if hoglin_spawns_as_baby(level.random(), group) {
                    set(&mut tag, "Age", Tag::Int(-24000));
                }
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:zoglin" => {
                if level.random().next_f32() < 0.2 {
                    set(&mut tag, "IsBaby", byte(true));
                }
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:piglin" => {
                let spawn = piglin_spawn(level.random(), &mut own);
                if spawn.baby {
                    set(&mut tag, "IsBaby", byte(true));
                }
                if let Some(weapon) = spawn.weapon {
                    equip(&mut tag, "mainhand", weapon);
                }
                minecraftoss_core::entity_data::set_brain_memory(&mut tag, "minecraft:hunted_recently", Tag::Byte(1), Some(i64::from(spawn.hunted)));
                for (slot, item) in spawn.armor.into_iter().flatten() {
                    equip(&mut tag, slot, item);
                }
                Self::roll_enchantments(level, &tag, difficulty);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:piglin_brute" => {
                minecraftoss_core::entity_data::finalize_piglin_brute(&mut tag, &self.dimension, [pos.x, pos.y, pos.z]);
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            "minecraft:creeper" | "minecraft:enderman" | "minecraft:bat" | "minecraft:ghast" | "minecraft:blaze" => finalize_mob(&mut tag, &mut RandomRef(level.random())),
            _ => {
                level.note_unsupported(&format!("finalizeSpawn of {kind}"));
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
        }
        tag
    }

    /// `AgeableMob.finalizeSpawn` for the ageable monsters.
    fn ageable_monster(level: &mut dyn SpawnLevel, tag: &mut Tag, group: &mut MonsterGroup) {
        if let MonsterGroup::Ageable { should_spawn_baby, chance, size } = group {
            if *should_spawn_baby && *size > 0 && level.random().next_f32() <= *chance {
                set(tag, "Age", Tag::Int(-24000));
            }
            *size += 1;
        }
    }

    /// `AbstractCubeMob.setSize(size, true)`: attributes by size, full health.
    fn set_cube_size(tag: &mut Tag, size: i32) {
        set(tag, "Size", Tag::Int(size - 1));
        let health = f64::from(size * size);
        set_attribute_base(tag, "minecraft:max_health", health);
        set_attribute_base(tag, "minecraft:movement_speed", f64::from(0.2f32 + 0.1 * size as f32));
        set_attribute_base(tag, "minecraft:attack_damage", f64::from(size));
        set(tag, "Health", Tag::Float(health as f32));
    }

    /// `AbstractSkeleton.finalizeSpawn` after `Mob`'s part.
    fn finalize_skeleton(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, tag: &mut Tag, difficulty: DifficultyInstance) {
        let wither = tag.get("id").and_then(Tag::as_str) == Some("minecraft:wither_skeleton");
        Self::populate_armor(level, tag, difficulty);
        equip(tag, "mainhand", if wither { "minecraft:stone_sword" } else { "minecraft:bow" });
        if !wither {
            Self::roll_enchantments(level, tag, difficulty);
        }
        let loot = level.random().next_f32() < 0.55 * difficulty.special_multiplier();
        set(tag, "CanPickUpLoot", byte(loot));
        Self::halloween_head(level, context, tag);
        if wither {
            set_attribute_base(tag, "minecraft:attack_damage", 4.0);
        }
    }

    /// `Zombie.finalizeSpawn` after `Mob`'s part: the entity to add is the
    /// zombie, or a new chicken it rides.
    #[allow(clippy::too_many_arguments)]
    fn finalize_zombie(
        &self,
        level: &mut dyn SpawnLevel,
        context: &SpawnContext,
        mut tag: Tag,
        pos: BlockPos,
        difficulty: DifficultyInstance,
        group: &mut MonsterGroup,
        own: &mut LegacyRandom,
    ) -> Tag {
        let special = difficulty.special_multiplier();
        let piglin = tag.get("id").and_then(Tag::as_str) == Some("minecraft:zombified_piglin");
        let drowned = tag.get("id").and_then(Tag::as_str) == Some("minecraft:drowned");
        let loot = level.random().next_f32() < 0.55 * special;
        set(&mut tag, "CanPickUpLoot", byte(loot));
        if matches!(group, MonsterGroup::None) {
            // `getSpawnAsBabyOdds`.
            let baby = level.random().next_f32() < 0.05;
            *group = MonsterGroup::Zombie { baby, can_spawn_jockey: true };
        }
        let mut vehicle = None;
        if let MonsterGroup::Zombie { baby, can_spawn_jockey } = *group {
            if baby {
                set(&mut tag, "IsBaby", byte(true));
                // `setBaby` touches the speed (the baby modifier is transient).
                let _ = attribute(&mut tag, "minecraft:movement_speed", f64::from(0.23f32));
                if can_spawn_jockey {
                    if f64::from(level.random().next_f32()) < 0.05 {
                        let at = Self::position(&tag);
                        let half = f64::from(0.49f32 / 2.0);
                        let search = [at[0] - half - 5.0, at[1] - 3.0, at[2] - half - 5.0, at[0] + half + 5.0, at[1] + f64::from(0.98f32) + 3.0, at[2] + half + 5.0];
                        if context.chickens.iter().any(|c| overlaps(&search, c)) {
                            level.note_unsupported("baby zombie riding a nearby chicken");
                        }
                    } else if f64::from(level.random().next_f32()) < 0.05 {
                        vehicle = self.jockey_chicken(level, &tag, pos);
                    }
                }
            }
            let doors = level.random().next_f32() < special * 0.1;
            set(&mut tag, "CanBreakDoors", byte(doors));
            if piglin {
                let item = if level.random().next_i32_bound(20) == 0 { "minecraft:golden_spear" } else { "minecraft:golden_sword" };
                equip(&mut tag, "mainhand", item);
            } else if drowned {
                if let Some(item) = drowned_weapon(level.random()) {
                    equip(&mut tag, "mainhand", item);
                }
            } else {
                Self::populate_armor(level, &mut tag, difficulty);
            }
            // `Zombie.populateDefaultEquipmentSlots`: a rare iron weapon.
            let weapon_chance = if difficulty.base == 3 { 0.05 } else { 0.01 };
            if !piglin && !drowned && level.random().next_f32() < weapon_chance {
                let item = match level.random().next_i32_bound(6) {
                    0 => "minecraft:iron_sword",
                    1 => "minecraft:iron_spear",
                    _ => "minecraft:iron_shovel",
                };
                equip(&mut tag, "mainhand", item);
            }
            Self::roll_enchantments(level, &tag, difficulty);
        }
        Self::halloween_head(level, context, &mut tag);
        // `handleAttributes`, from the zombie's own random.
        Self::zombie_attributes(&mut tag, own, special, piglin);
        match vehicle {
            Some(mut chicken) => {
                set(&mut chicken, "Passengers", Tag::List(vec![tag]));
                chicken
            }
            None => tag,
        }
    }

    fn zombie_attributes(tag: &mut Tag, own: &mut LegacyRandom, special: f32, piglin: bool) {
        let reinforcements = if piglin { 0.0 } else { own.next_f64() * f64::from(0.1f32) };
        set_attribute_base(tag, "minecraft:spawn_reinforcements", reinforcements);
        add_modifier(tag, "minecraft:knockback_resistance", 0.0, "minecraft:random_spawn_bonus", own.next_f64() * f64::from(0.05f32), "add_value");
        let follow = own.next_f64() * 1.5 * f64::from(special);
        if follow > 1.0 {
            add_modifier(tag, "minecraft:follow_range", 35.0, "minecraft:zombie_random_spawn_bonus", follow, "add_multiplied_total");
        }
        if own.next_f32() < special * 0.05 {
            add_modifier(tag, "minecraft:spawn_reinforcements", 0.0, "minecraft:leader_zombie_bonus", own.next_f64() * 0.25 + 0.5, "add_value");
            let bonus = own.next_f64() * 3.0 + 1.0;
            add_modifier(tag, "minecraft:max_health", 20.0, "minecraft:leader_zombie_bonus", bonus, "add_multiplied_total");
            set(tag, "Health", Tag::Float((20.0 * (1.0 + bonus)).clamp(1.0, 1024.0) as f32));
            set(tag, "CanBreakDoors", byte(true));
        }
    }

    pub fn finalize_conversion(&self, level: &mut dyn SpawnLevel, context: &mut SpawnContext, mut tag: Tag) -> Tag {
        let at = Self::position(&tag);
        let pos = BlockPos::new(at[0].floor() as i32, at[1].floor() as i32, at[2].floor() as i32);
        if tag.get("UUID").is_none() {
            let uuid = level.next_uuid();
            set(&mut tag, "UUID", Tag::IntArray(uuid.to_vec()));
        }
        let kind = tag.get("id").and_then(Tag::as_str).map(str::to_owned);
        match kind.as_deref() {
            Some("minecraft:zombie_villager") => {
                set(&mut tag, "VillagerDataFinalized", byte(true));
                let _ = attribute(&mut tag, "minecraft:follow_range", follow_range_base("minecraft:zombie_villager"));
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
                let special = Self::difficulty_at(level, context, pos).special_multiplier();
                let doors = level.random().next_f32() < special * 0.1;
                set(&mut tag, "CanBreakDoors", byte(doors));
                Self::halloween_head(level, context, &mut tag);
                let mut own = entity_random(Self::uuid(&tag));
                Self::zombie_attributes(&mut tag, &mut own, special, false);
            }
            Some("minecraft:villager") => {
                if tag.get("VillagerDataFinalized").and_then(Tag::as_i64).unwrap_or(0) == 0 {
                    let biome = self.biome_name(level.biome(pos));
                    if let Tag::Compound(map) = &mut tag {
                        if let Some(Tag::Compound(data)) = map.get_mut("VillagerData") {
                            data.insert("type".to_owned(), Tag::String(villager_type(&biome).to_owned()));
                        }
                    }
                    set(&mut tag, "VillagerDataFinalized", byte(true));
                }
                finalize_mob(&mut tag, &mut RandomRef(level.random()));
            }
            Some(_) => finalize_mob(&mut tag, &mut RandomRef(level.random())),
            None => {}
        }
        tag
    }

    /// `Husk.finalizeSpawn` after the zombie's part: a second loot roll,
    /// then, for a natural spawn with room for one, a one-in-ten camel
    /// husk the husk rides with an iron spear, a parched riding behind
    /// (each finalized as a spawn of its own).
    fn finalize_husk(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, mut root: Tag, pos: BlockPos, difficulty: DifficultyInstance, natural: bool) -> Tag {
        let loot = level.random().next_f32() < 0.55 * difficulty.special_multiplier();
        let on_its_own = root.get("id").and_then(Tag::as_str) == Some("minecraft:husk");
        if !on_its_own {
            // A baby husk on a chicken: the roll lands on the rider.
            if let Some(Tag::List(riders)) = root.get("Passengers").cloned() {
                if let Some(mut husk) = riders.into_iter().next() {
                    set(&mut husk, "CanPickUpLoot", byte(loot));
                    set(&mut root, "Passengers", Tag::List(vec![husk]));
                }
            }
            level.note_unsupported("camel husk for a husk jockey");
            return root;
        }
        set(&mut root, "CanPickUpLoot", byte(loot));
        if !natural {
            return root;
        }
        let at = Self::position(&root);
        let (x, z) = (f64::from(pos.x) + 0.5, f64::from(pos.z) + 0.5);
        let Some(camel_info) = type_info("minecraft:camel_husk") else { return root };
        if !self.no_collision(level, spawn_aabb(&camel_info, x, f64::from(pos.y), z)) {
            return root;
        }
        if level.random().next_f32() >= 0.1 {
            return root;
        }
        let catalog = self.registries.entities.as_ref();
        let (Some(mut camel), Some(mut parched)) = (catalog.and_then(|c| c.default_tag("minecraft:camel_husk")), catalog.and_then(|c| c.default_tag("minecraft:parched"))) else {
            return root;
        };
        equip(&mut root, "mainhand", "minecraft:iron_spear");
        // `Camel.finalizeSpawn`: no memories to draw, then `AbstractHorse`
        // (no attributes to roll), `AgeableMob` (a first-of-group, no baby
        // roll) and `Mob`. The camel is only `setPos`: it keeps its
        // constructor's yaw (its own unseeded random), and its riders take
        // its previous-tick rotation, still 0 (`AbstractHorse.addPassenger`'s
        // `absSnapRotationTo(getViewYRot(0))`).
        let camel_uuid = level.next_uuid();
        let camel_yaw = entity_random(camel_uuid).next_f32() * std::f64::consts::TAU as f32;
        place(&mut camel, at, camel_yaw, 0.0, camel_uuid);
        set(&mut root, "Rotation", Tag::List(vec![Tag::Float(0.0), Tag::Float(0.0)]));
        let _ = attribute(&mut camel, "minecraft:follow_range", 16.0);
        finalize_mob(&mut camel, &mut RandomRef(level.random()));
        // The parched: a skeleton's finalize.
        let parched_uuid = level.next_uuid();
        place(&mut parched, at, 0.0, 0.0, parched_uuid);
        let _ = attribute(&mut parched, "minecraft:follow_range", 16.0);
        finalize_mob(&mut parched, &mut RandomRef(level.random()));
        self.finalize_skeleton(level, context, &mut parched, difficulty);
        set(&mut camel, "Passengers", Tag::List(vec![root, parched]));
        camel
    }

    fn finalize_drowned(&self, level: &mut dyn SpawnLevel, mut root: Tag, pos: BlockPos, natural: bool) -> Tag {
        let biome = level.biome(pos);
        let frequent = self.more_frequent_drowned.is_some_and(|t| self.registries.biome_in_tag(biome, t));
        let Some(drowned) = drowned_of(&mut root) else { return root };
        if !has_equipment(drowned, "offhand") && drowned_shell(level.random()) {
            equip(drowned, "offhand", "minecraft:nautilus_shell");
            let chances = [("offhand".to_owned(), Tag::Float(2.0))].into_iter().collect();
            set(drowned, "drop_chances", Tag::Compound(chances));
        }
        let trident = drowned.get("equipment").and_then(|e| e.get("mainhand")).and_then(|i| i.get("id")).and_then(Tag::as_str) == Some("minecraft:trident");
        let baby = drowned.get("IsBaby").and_then(Tag::as_i64) == Some(1);
        if natural && trident && level.random().next_f32() < 0.5 && !baby && !frequent {
            level.note_unsupported("zombie nautilus jockey");
        }
        root
    }

    fn finalize_strider(&self, level: &mut dyn SpawnLevel, context: &SpawnContext, mut tag: Tag, pos: BlockPos, difficulty: DifficultyInstance) -> Tag {
        let baby = tag.get("Age").and_then(Tag::as_i64).is_some_and(|age| age < 0);
        let mut rider = None;
        if !baby {
            let catalog = self.registries.entities.as_ref();
            if level.random().next_i32_bound(30) == 0 && context.spawning_monsters {
                if let Some(mut piglin) = catalog.and_then(|c| c.default_tag("minecraft:zombified_piglin")) {
                    let uuid = level.next_uuid();
                    let mut group = MonsterGroup::Zombie { baby: level.random().next_f32() < 0.05, can_spawn_jockey: false };
                    place(&mut piglin, Self::position(&tag), Self::y_rot(&tag), 0.0, uuid);
                    let mut piglin = self.finalize_monster(level, context, "minecraft:zombified_piglin", piglin, pos, difficulty, &mut group, false);
                    equip(&mut piglin, "mainhand", "minecraft:warped_fungus_on_a_stick");
                    equip(&mut tag, "saddle", "minecraft:saddle");
                    let chances = [("saddle".to_owned(), Tag::Float(2.0))].into_iter().collect();
                    set(&mut tag, "drop_chances", Tag::Compound(chances));
                    rider = Some(piglin);
                }
            } else if level.random().next_i32_bound(10) == 0
                && let Some(mut child) = catalog.and_then(|c| c.default_tag("minecraft:strider"))
            {
                let uuid = level.next_uuid();
                set(&mut child, "Age", Tag::Int(-24000));
                place(&mut child, Self::position(&tag), Self::y_rot(&tag), 0.0, uuid);
                self.finalize_spawn(level, "minecraft:strider", &mut child, pos, &mut Group::None);
                rider = Some(child);
            }
        }
        self.finalize_spawn(level, "minecraft:strider", &mut tag, pos, &mut Group::None);
        if let Some(rider) = rider {
            set(&mut tag, "Passengers", Tag::List(vec![rider]));
        }
        tag
    }

    /// A chicken jockey's new chicken at the zombie, finalized as a spawn
    /// of its own.
    fn jockey_chicken(&self, level: &mut dyn SpawnLevel, zombie: &Tag, pos: BlockPos) -> Option<Tag> {
        let mut chicken = self.registries.entities.as_ref().and_then(|c| c.default_tag("minecraft:chicken"))?;
        let uuid = level.next_uuid();
        place(&mut chicken, Self::position(zombie), Self::y_rot(zombie), 0.0, uuid);
        self.finalize_spawn(level, "minecraft:chicken", &mut chicken, pos, &mut Group::None);
        set(&mut chicken, "IsChickenJockey", byte(true));
        Some(chicken)
    }

    fn uuid(tag: &Tag) -> [i32; 4] {
        match tag.get("UUID") {
            Some(Tag::IntArray(v)) if v.len() == 4 => [v[0], v[1], v[2], v[3]],
            _ => [0; 4],
        }
    }

    fn position(tag: &Tag) -> [f64; 3] {
        let at = |i: usize| match tag.get("Pos") {
            Some(Tag::List(p)) => match p.get(i) {
                Some(Tag::Double(v)) => *v,
                _ => 0.0,
            },
            _ => 0.0,
        };
        [at(0), at(1), at(2)]
    }

    fn y_rot(tag: &Tag) -> f32 {
        match tag.get("Rotation") {
            Some(Tag::List(r)) => match r.first() {
                Some(Tag::Float(v)) => *v,
                _ => 0.0,
            },
            _ => 0.0,
        }
    }

    fn biome_name(&self, biome: BiomeId) -> String {
        self.registries.biomes.get(biome).name.as_str().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::MobCategory;

    #[test]
    fn a_hoglin_group_rolls_one_in_five_babies_and_a_further_twentieth_after_the_first() {
        let mut random = minecraftoss_core::random::LegacyRandom::new(42);
        let groups = 20_000;
        let (mut first, mut later) = (0, 0);
        for _ in 0..groups {
            let mut group = super::MonsterGroup::None;
            for member in 0..4 {
                if super::hoglin_spawns_as_baby(&mut random, &mut group) {
                    if member == 0 {
                        first += 1;
                    } else {
                        later += 1;
                    }
                }
            }
        }
        let first = f64::from(first) / f64::from(groups);
        let later = f64::from(later) / f64::from(groups * 3);
        assert!((first - 0.2).abs() < 0.01, "{first}");
        assert!((later - 0.24).abs() < 0.01, "{later}");
        let info = super::type_info("minecraft:hoglin").unwrap();
        assert_eq!((info.width, info.height, info.cluster, info.fire_immune, info.remove_far), (1.396_484_4, 1.4, 4, false, true));
        assert_eq!(info.rules, super::Rules::Hoglin);
        let zoglin = super::type_info("minecraft:zoglin").unwrap();
        assert_eq!((zoglin.fire_immune, zoglin.remove_far, zoglin.rules), (true, true, super::Rules::AnyLightMonster));
    }

    #[test]
    fn ocelots_spawn_by_their_own_rules_and_cats_have_none() {
        let ocelot = super::type_info("minecraft:ocelot").unwrap();
        assert_eq!((ocelot.width, ocelot.height, ocelot.cluster, ocelot.fire_immune, ocelot.remove_far), (0.6, 0.7, 4, false, false));
        assert_eq!((ocelot.placement, ocelot.rules), (crate::natural_spawner::Placement::OnGround, super::Rules::Ocelot));
        assert!(matches!(ocelot.heightmap, minecraftoss_core::chunk::HeightmapKind::MotionBlocking));
        assert!(matches!(ocelot.walk, super::Walk::Animal) && ocelot.rules.creature());
        let cat = super::type_info("minecraft:cat").unwrap();
        assert_eq!((cat.width, cat.height, cat.remove_far), (0.6, 0.7, false));
        assert_eq!((cat.placement, cat.rules), (crate::natural_spawner::Placement::NoRestrictions, super::Rules::Unrestricted));
        assert!(cat.rules.creature());
        assert_eq!(super::BLACK_CAT_STRUCTURES, ["minecraft:swamp_hut"]);
    }

    #[test]
    fn a_cave_spider_spawns_by_the_spider_rules_at_its_own_size() {
        let spider = super::type_info("minecraft:spider").unwrap();
        let cave = super::type_info("minecraft:cave_spider").unwrap();
        assert_eq!((cave.width, cave.height), (0.7, 0.5));
        assert_eq!((cave.rules, cave.walk, cave.remove_far, cave.fire_immune), (spider.rules, spider.walk, spider.remove_far, spider.fire_immune));
        assert_eq!(cave.rules, super::Rules::Monster);
        assert_eq!(crate::natural_spawner::type_width("minecraft:cave_spider"), Some(0.7));
    }

    #[test]
    fn a_piglin_spawn_rolls_a_baby_one_in_five_and_a_crossbow_for_half_the_adults() {
        let mut level = minecraftoss_core::random::LegacyRandom::new(11);
        let mut own = minecraftoss_core::random::LegacyRandom::new(12);
        let rolls = 10_000;
        let (mut babies, mut crossbows, mut spears, mut swords) = (0, 0, 0, 0);
        let mut pieces = [0; 4];
        for _ in 0..rolls {
            let spawn = super::piglin_spawn(&mut level, &mut own);
            assert!((600..=2400).contains(&spawn.hunted));
            if spawn.baby {
                babies += 1;
                assert!(spawn.weapon.is_none() && spawn.armor.iter().all(Option::is_none));
                continue;
            }
            match spawn.weapon {
                Some("minecraft:crossbow") => crossbows += 1,
                Some("minecraft:golden_spear") => spears += 1,
                Some("minecraft:golden_sword") => swords += 1,
                other => panic!("{other:?}"),
            }
            for (count, piece) in pieces.iter_mut().zip(spawn.armor) {
                if piece.is_some() {
                    *count += 1;
                }
            }
        }
        let adults = f64::from(rolls - babies);
        let share = |n: i32, of: f64| f64::from(n) / of;
        assert!((share(babies, f64::from(rolls)) - 0.2).abs() < 0.015, "{babies}");
        assert!((share(crossbows, adults) - 0.5).abs() < 0.02, "{crossbows}");
        assert!((share(spears, adults) - 0.05).abs() < 0.01, "{spears}");
        assert!((share(swords, adults) - 0.45).abs() < 0.02, "{swords}");
        for count in pieces {
            assert!((share(count, adults) - 0.1).abs() < 0.015, "{pieces:?}");
        }
        let info = super::type_info("minecraft:piglin").unwrap();
        assert_eq!((info.width, info.height, info.fire_immune, info.remove_far, info.rules), (0.6, 1.95, false, true, super::Rules::Piglin));
        assert_eq!(super::follow_range_base("minecraft:piglin_brute"), 12.0);
        assert_eq!(super::follow_range_base("minecraft:piglin"), 16.0);
    }

    #[test]
    fn piglins_do_not_spawn_on_nether_wart_blocks() {
        assert!(!super::piglin_may_stand_on("minecraft:nether_wart_block"));
        for floor in ["minecraft:netherrack", "minecraft:crimson_nylium", "minecraft:warped_wart_block", "minecraft:soul_sand"] {
            assert!(super::piglin_may_stand_on(floor), "{floor}");
        }
        let brute = super::type_info("minecraft:piglin_brute").unwrap();
        assert_eq!((brute.rules, brute.remove_far), (super::Rules::Other, true));
    }

    #[test]
    fn every_nether_monster_has_the_monster_category() {
        for kind in ["blaze", "ghast", "magma_cube", "piglin", "piglin_brute", "hoglin", "zoglin", "wither_skeleton", "zombified_piglin", "skeleton", "enderman"] {
            assert_eq!(MobCategory::of_type(&format!("minecraft:{kind}")), Some(MobCategory::Monster), "{kind}");
        }
        assert_eq!(MobCategory::of_type("minecraft:strider"), Some(MobCategory::Creature));
    }

    #[test]
    fn a_ghast_spawns_alone_in_its_four_block_box_on_any_floor_and_goes_when_far() {
        let info = super::type_info("minecraft:ghast").unwrap();
        assert_eq!((info.width, info.height, info.cluster, info.fire_immune, info.remove_far), (4.0, 4.0, 1, true, true));
        assert_eq!(info.rules, super::Rules::Ghast);
        assert!(matches!(info.walk, super::Walk::Neutral));
        assert_eq!(super::follow_range_base("minecraft:ghast"), 100.0);
    }

    struct Column {
        solid_top: i32,
        water_top: i32,
        floor: minecraftoss_core::BlockStateId,
        water: minecraftoss_core::BlockStateId,
        biome: minecraftoss_core::BiomeId,
        darken: i32,
        random: minecraftoss_core::random::LegacyRandom,
        made: i32,
        added: Vec<minecraftoss_core::nbt::Tag>,
    }

    impl super::SpawnLevel for Column {
        fn block(&self, pos: minecraftoss_core::BlockPos) -> minecraftoss_core::BlockStateId {
            if pos.y <= self.solid_top {
                self.floor
            } else if pos.y <= self.water_top {
                self.water
            } else {
                minecraftoss_core::BlockStateId::AIR
            }
        }
        fn height(&self, _kind: minecraftoss_core::chunk::HeightmapKind, _x: i32, _z: i32) -> i32 {
            self.water_top.max(self.solid_top) + 1
        }
        fn biome(&self, _pos: minecraftoss_core::BlockPos) -> minecraftoss_core::BiomeId {
            self.biome
        }
        fn noise_biome(&self, _qx: i32, _qy: i32, _qz: i32) -> minecraftoss_core::BiomeId {
            self.biome
        }
        fn raw_brightness(&mut self, pos: minecraftoss_core::BlockPos, darkening: i32) -> i32 {
            (self.sky_brightness(pos) - darkening).max(0)
        }
        fn sky_brightness(&mut self, pos: minecraftoss_core::BlockPos) -> i32 {
            if pos.y <= self.solid_top {
                0
            } else if pos.y <= self.water_top {
                (15 - (self.water_top + 1 - pos.y)).max(0)
            } else {
                15
            }
        }
        fn block_brightness(&mut self, _pos: minecraftoss_core::BlockPos) -> i32 {
            0
        }
        fn sky_darken(&self) -> i32 {
            self.darken
        }
        fn min_y(&self) -> i32 {
            -64
        }
        fn random(&mut self) -> &mut dyn minecraftoss_core::random::RandomSource {
            &mut self.random
        }
        fn next_uuid(&mut self) -> [i32; 4] {
            self.made += 1;
            [self.made, 0, 0, 1]
        }
        fn add_entity(&mut self, entity: minecraftoss_core::nbt::Tag) {
            self.added.push(entity);
        }
    }

    fn river() -> Option<(std::sync::Arc<super::CreatureSpawns>, Column)> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        registries.entities.as_ref()?;
        let biome = registries.biomes.id("minecraft:river")?;
        let floor = registries.blocks.parse_state("minecraft:sand").ok()?;
        let water = registries.blocks.parse_state("minecraft:water").ok()?;
        let spawns = super::CreatureSpawns::load(std::sync::Arc::new(registries), "minecraft:overworld", false).ok()?;
        let column = Column { solid_top: 49, water_top: 62, floor, water, biome, darken: 11, random: minecraftoss_core::random::LegacyRandom::new(9), made: 0, added: Vec::new() };
        Some((std::sync::Arc::new(spawns), column))
    }

    fn spawn_monsters(spawns: &super::CreatureSpawns, column: &mut Column, y: i32, rounds: usize) {
        let players = [[8.5, 63.0, 48.5]];
        for _ in 0..rounds {
            let mut context = super::SpawnContext {
                players: &players,
                respawn: None,
                difficulty: 2,
                overworld_time: 0,
                moon_brightness: 0.0,
                raining: false,
                thundering: false,
                seed: 0,
                surface_slime_chance: &|_| 0.0,
                halloween: false,
                can_spawn_in_chunk: &|_| true,
                obstacles: Vec::new(),
                chickens: Vec::new(),
                structures: &|_| Vec::new(),
                spawning_monsters: true,
            };
            spawns.spawn_category_for_position(column, &mut context, &mut super::Unlimited, MobCategory::Monster, minecraftoss_core::ChunkPos::new(0, 0), minecraftoss_core::BlockPos::new(8, y, 8));
        }
    }

    #[test]
    fn a_drowned_spawns_in_a_river_column_at_night_and_never_on_dry_land() {
        let Some((spawns, mut column)) = river() else { return };
        spawn_monsters(&spawns, &mut column, 55, 2000);
        let kinds: Vec<&str> = column.added.iter().filter_map(|tag| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str)).collect();
        assert!(kinds.len() > 20, "{}", kinds.len());
        assert!(kinds.iter().all(|kind| *kind == "minecraft:drowned"), "{kinds:?}");
        for tag in &column.added {
            let y = tag.get("Pos").and_then(minecraftoss_core::nbt::Tag::as_list).and_then(|p| p.get(1)).and_then(minecraftoss_core::nbt::Tag::as_f64).unwrap();
            assert!((50.0..=62.0).contains(&y), "{y}");
        }
        let mut dry = Column { solid_top: 64, water_top: 64, added: Vec::new(), ..column };
        spawn_monsters(&spawns, &mut dry, 65, 2000);
        assert!(!dry.added.is_empty(), "other monsters spawn on the bank");
        assert!(dry.added.iter().all(|tag| tag.get("id").and_then(minecraftoss_core::nbt::Tag::as_str) != Some("minecraft:drowned")));
        let info = super::type_info("minecraft:drowned").unwrap();
        assert_eq!((info.width, info.height, info.placement, info.rules, info.walk, info.remove_far, info.cluster), (0.6, 1.95, super::super::Placement::InWater, super::Rules::Drowned, super::Walk::Monster, true, 4));
        let mut puddle = Column { solid_top: 61, water_top: 62, added: Vec::new(), ..dry };
        let context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 0,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        let at = minecraftoss_core::BlockPos::new(8, 62, 8);
        assert!((0..500).all(|_| !spawns.natural_spawn_rules(&mut puddle, &context, "minecraft:drowned", &info, at)), "a drowned needs water below its feet");
    }

    #[test]
    fn a_drowned_rolls_a_trident_or_fishing_rod_one_time_in_ten_and_a_nautilus_shell_three_in_a_hundred() {
        let mut random = minecraftoss_core::random::LegacyRandom::new(21);
        let rolls = 40_000;
        let (mut tridents, mut rods, mut shells) = (0, 0, 0);
        for _ in 0..rolls {
            match super::drowned_weapon(&mut random) {
                Some("minecraft:trident") => tridents += 1,
                Some("minecraft:fishing_rod") => rods += 1,
                None => {}
                other => panic!("{other:?}"),
            }
            if super::drowned_shell(&mut random) {
                shells += 1;
            }
        }
        let share = |n: i32| f64::from(n) / f64::from(rolls);
        assert!((share(tridents) - 0.1 * 10.0 / 16.0).abs() < 0.005, "{tridents}");
        assert!((share(rods) - 0.1 * 6.0 / 16.0).abs() < 0.005, "{rods}");
        assert!((share(shells) - 0.03).abs() < 0.004, "{shells}");
    }

    #[test]
    fn a_blaze_spawns_as_a_monster_in_any_light() {
        let info = super::type_info("minecraft:blaze").unwrap();
        assert_eq!((info.width, info.height, info.cluster, info.fire_immune, info.remove_far), (0.6, 1.8, 4, true, true));
        assert_eq!(info.rules, super::Rules::AnyLightMonster);
        assert!(matches!(info.walk, super::Walk::Monster));
        assert_eq!(super::follow_range_base("minecraft:blaze"), 48.0);
    }

    #[test]
    fn a_patrol_is_a_captain_with_the_ominous_banner_and_pillagers_on_patrol() {
        use minecraftoss_core::nbt::Tag;
        let Some((spawns, column)) = river() else { return };
        let mut land = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let mut context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        let members = spawns.spawn_patrol(&mut land, &mut context, minecraftoss_core::BlockPos::new(30, 90, -40));
        assert_eq!(members.len(), 3, "ceil(1.5) + 1 on a new world on normal");
        let flag = |tag: &Tag, key: &str| tag.get(key).and_then(Tag::as_i64) == Some(1);
        let captain = &members[0];
        assert!(flag(captain, "PatrolLeader") && flag(captain, "Patrolling"));
        let target = captain.get("patrol_target").and_then(Tag::as_ints).expect("a patrol target");
        assert!(target[0].abs() <= 500 && target[2].abs() <= 500 && target[1] == 0, "found before it is placed: {target:?}");
        let head = captain.get("equipment").and_then(|e| e.get("head")).expect("the banner");
        assert_eq!(head.get("id").and_then(Tag::as_str), Some("minecraft:white_banner"));
        let components = head.get("components").expect("banner components");
        assert_eq!(components.get("minecraft:banner_patterns").and_then(Tag::as_list).map(<[Tag]>::len), Some(8));
        assert_eq!(components.get("minecraft:item_name").and_then(|n| n.get("translate")).and_then(Tag::as_str), Some(super::OMINOUS_BANNER_NAME));
        assert_eq!(components.get("minecraft:rarity").and_then(Tag::as_str), Some("uncommon"));
        assert_eq!(captain.get("drop_chances").and_then(|c| c.get("head")).and_then(Tag::as_f64), Some(2.0));
        let pos = |tag: &Tag| tag.get("Pos").and_then(Tag::as_list).map(|p| p.iter().filter_map(Tag::as_f64).collect::<Vec<f64>>()).unwrap();
        assert_eq!(pos(captain), [30.0, 65.0, -40.0], "on the ground at the block corner");
        for member in &members[1..] {
            assert!(flag(member, "Patrolling") && !flag(member, "PatrolLeader"));
            assert!(member.get("patrol_target").is_none());
            assert!(member.get("equipment").and_then(|e| e.get("head")).is_none());
            let at = pos(member);
            assert!((at[0] - 30.0).abs() <= 8.0 && (at[2] + 40.0).abs() <= 8.0 && at[1] == 65.0, "{at:?}");
        }
        for member in &members {
            assert_eq!(member.get("equipment").and_then(|e| e.get("mainhand")).and_then(|h| h.get("id")).and_then(Tag::as_str), Some("minecraft:crossbow"));
            assert!(flag(member, "CanJoinRaid"));
        }
        context.difficulty = 0;
        assert!(spawns.spawn_patrol(&mut land, &mut context, minecraftoss_core::BlockPos::new(30, 90, -40)).is_empty(), "no patrol in peaceful");
        context.difficulty = 2;
        let mut flooded = Column { solid_top: 60, water_top: 64, ..land };
        assert!(spawns.spawn_patrol(&mut flooded, &mut context, minecraftoss_core::BlockPos::new(30, 90, -40)).is_empty(), "no captain in water, no patrol");
    }

    #[test]
    fn a_summoned_vindicator_carries_an_iron_axe_and_sometimes_leads_with_the_banner() {
        use minecraftoss_core::nbt::Tag;
        let info = super::type_info("minecraft:vindicator").unwrap();
        assert_eq!((info.width, info.height, info.placement, info.rules, info.remove_far), (0.6, 1.95, super::super::Placement::NoRestrictions, super::Rules::Monster, true));
        assert_eq!(MobCategory::of_type("minecraft:vindicator"), Some(MobCategory::Monster));
        assert_eq!(super::follow_range_base("minecraft:vindicator"), 12.0);
        let Some((spawns, column)) = river() else { return };
        let mut land = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let mut context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        let mut leaders = 0;
        for _ in 0..600 {
            let tag = spawns.summon(&mut land, &mut context, "minecraft:vindicator", [0.5, 65.0, 0.5], None, 0.0).expect("a vindicator");
            assert_eq!(tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|h| h.get("id")).and_then(Tag::as_str), Some("minecraft:iron_axe"));
            assert_eq!(tag.get("CanJoinRaid").and_then(Tag::as_i64), Some(1));
            assert_ne!(tag.get("Patrolling").and_then(Tag::as_i64), Some(1));
            if tag.get("PatrolLeader").and_then(Tag::as_i64) == Some(1) {
                leaders += 1;
                assert_eq!(tag.get("equipment").and_then(|e| e.get("head")).and_then(|h| h.get("id")).and_then(Tag::as_str), Some("minecraft:white_banner"));
            }
        }
        assert!((15..=60).contains(&leaders), "about 6 %: {leaders}");
    }

    #[test]
    fn a_summoned_evoker_holds_nothing_can_join_raids_and_sometimes_leads() {
        use minecraftoss_core::nbt::Tag;
        let info = super::type_info("minecraft:evoker").unwrap();
        assert_eq!((info.width, info.height, info.placement, info.rules, info.remove_far), (0.6, 1.95, super::super::Placement::NoRestrictions, super::Rules::Monster, true));
        assert_eq!(MobCategory::of_type("minecraft:evoker"), Some(MobCategory::Monster));
        assert_eq!(super::follow_range_base("minecraft:evoker"), 12.0);
        let Some((spawns, column)) = river() else { return };
        let mut land = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let mut context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        let mut leaders = 0;
        for _ in 0..600 {
            let tag = spawns.summon(&mut land, &mut context, "minecraft:evoker", [0.5, 65.0, 0.5], None, 0.0).expect("an evoker");
            assert!(tag.get("equipment").and_then(|e| e.get("mainhand")).is_none());
            assert_eq!(tag.get("CanJoinRaid").and_then(Tag::as_i64), Some(1));
            if tag.get("PatrolLeader").and_then(Tag::as_i64) == Some(1) {
                leaders += 1;
            }
        }
        assert!((15..=60).contains(&leaders), "about 6 %: {leaders}");
    }

    #[test]
    fn a_summoned_ravager_can_join_raids_but_never_leads() {
        use minecraftoss_core::nbt::Tag;
        let info = super::type_info("minecraft:ravager").unwrap();
        assert_eq!((info.width, info.height, info.placement, info.fire_immune, info.rules), (1.95, 2.2, super::super::Placement::OnGround, false, super::Rules::Monster));
        assert_eq!(MobCategory::of_type("minecraft:ravager"), Some(MobCategory::Monster));
        assert_eq!(super::follow_range_base("minecraft:ravager"), 32.0);
        let Some((spawns, column)) = river() else { return };
        let mut land = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let mut context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        for _ in 0..60 {
            let tag = spawns.summon(&mut land, &mut context, "minecraft:ravager", [0.5, 65.0, 0.5], None, 0.0).expect("a ravager");
            assert_eq!(tag.get("CanJoinRaid").and_then(Tag::as_i64), Some(1));
            assert_ne!(tag.get("PatrolLeader").and_then(Tag::as_i64), Some(1), "a ravager never leads");
            assert!(tag.get("equipment").and_then(|e| e.get("head")).is_none());
        }
    }

    #[test]
    fn a_summoned_vex_carries_an_iron_sword_it_never_drops() {
        use minecraftoss_core::nbt::Tag;
        let info = super::type_info("minecraft:vex").unwrap();
        assert_eq!((info.width, info.height, info.placement, info.fire_immune, info.rules), (0.4, 0.8, super::super::Placement::NoRestrictions, true, super::Rules::Monster));
        assert_eq!(MobCategory::of_type("minecraft:vex"), Some(MobCategory::Monster));
        let Some((spawns, column)) = river() else { return };
        let mut land = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let mut context = super::SpawnContext {
            players: &[],
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        let tag = spawns.summon(&mut land, &mut context, "minecraft:vex", [0.5, 65.0, 0.5], None, 0.0).expect("a vex");
        assert_eq!(tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|h| h.get("id")).and_then(Tag::as_str), Some("minecraft:iron_sword"));
        assert_eq!(tag.get("drop_chances").and_then(|c| c.get("mainhand")).and_then(Tag::as_f64), Some(0.0));
        assert!(tag.get("life_ticks").is_none(), "a summoned vex lives on");
    }

    #[test]
    fn pillagers_spawn_alone_in_a_pillager_outpost_by_day_with_a_crossbow() {
        use minecraftoss_core::nbt::Tag;
        let info = super::type_info("minecraft:pillager").unwrap();
        assert_eq!((info.width, info.height, info.cluster, info.fire_immune, info.remove_far), (0.6, 1.95, 1, false, true));
        assert_eq!(info.rules, super::Rules::Patrolling);
        assert!(matches!(info.walk, super::Walk::Neutral));
        assert_eq!(super::follow_range_base("minecraft:pillager"), 32.0);
        let Some((spawns, column)) = river() else { return };
        let mut day = Column { solid_top: 64, water_top: 64, darken: 0, added: Vec::new(), ..column };
        let players = [[8.5, 65.0, 48.5]];
        let outpost = |_: minecraftoss_core::BlockPos| vec![super::StructureHere { name: "minecraft:pillager_outpost".to_owned(), in_box: true, in_piece: false }];
        let rounds = 3000;
        for _ in 0..rounds {
            let mut context = super::SpawnContext {
                players: &players,
                respawn: None,
                difficulty: 2,
                overworld_time: 6000,
                moon_brightness: 0.0,
                raining: false,
                thundering: false,
                seed: 0,
                surface_slime_chance: &|_| 0.0,
                halloween: false,
                can_spawn_in_chunk: &|_| true,
                obstacles: Vec::new(),
                chickens: Vec::new(),
                structures: &outpost,
                spawning_monsters: true,
            };
            let before = day.added.len();
            spawns.spawn_category_for_position(&mut day, &mut context, &mut super::Unlimited, MobCategory::Monster, minecraftoss_core::ChunkPos::new(0, 0), minecraftoss_core::BlockPos::new(8, 65, 8));
            assert!(day.added.len() - before <= 1, "pillagers spawn alone");
        }
        let kinds: Vec<&str> = day.added.iter().filter_map(|tag| tag.get("id").and_then(Tag::as_str)).collect();
        assert!(kinds.len() > 100, "{}", kinds.len());
        assert!(kinds.iter().all(|kind| *kind == "minecraft:pillager"), "{kinds:?}");
        let leaders = day.added.iter().filter(|tag| tag.get("PatrolLeader").and_then(Tag::as_i64) == Some(1)).count();
        let share = leaders as f64 / kinds.len() as f64;
        assert!(share > 0.0 && share < 0.15, "{leaders} of {}", kinds.len());
        for tag in &day.added {
            let hand = tag.get("equipment").and_then(|e| e.get("mainhand")).and_then(|h| h.get("id")).and_then(Tag::as_str);
            assert_eq!(hand, Some("minecraft:crossbow"));
            assert_eq!(tag.get("CanJoinRaid").and_then(Tag::as_i64), Some(1));
            let leader = tag.get("PatrolLeader").and_then(Tag::as_i64) == Some(1);
            let head = tag.get("equipment").and_then(|e| e.get("head")).and_then(|h| h.get("id")).and_then(Tag::as_str);
            assert_eq!(head, leader.then_some("minecraft:white_banner"), "a leader carries the ominous banner");
            assert_ne!(tag.get("Patrolling").and_then(Tag::as_i64), Some(1), "an outpost pillager is no patroller");
        }
        let mut elsewhere = Column { added: Vec::new(), ..day };
        let mut context = super::SpawnContext {
            players: &players,
            respawn: None,
            difficulty: 2,
            overworld_time: 6000,
            moon_brightness: 0.0,
            raining: false,
            thundering: false,
            seed: 0,
            surface_slime_chance: &|_| 0.0,
            halloween: false,
            can_spawn_in_chunk: &|_| true,
            obstacles: Vec::new(),
            chickens: Vec::new(),
            structures: &|_| Vec::new(),
            spawning_monsters: true,
        };
        for _ in 0..500 {
            spawns.spawn_category_for_position(&mut elsewhere, &mut context, &mut super::Unlimited, MobCategory::Monster, minecraftoss_core::ChunkPos::new(0, 0), minecraftoss_core::BlockPos::new(8, 65, 8));
        }
        assert!(elsewhere.added.is_empty(), "no monsters by day outside the outpost");
    }
}
