use std::collections::BTreeMap;

use minecraft_net::Dimension;
use minecraft_terrain::game_mode::PlayerMode;
pub use minecraft_terrain::item_nbt::{slot_list, slots, stack_of, stack_tag};
use minecraft_terrain::level_storage::{uuid_from_ints, uuid_to_ints};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::survival::{EffectKind, SurvivalStatus};

use crate::guns::HOTBAR;

pub const STORAGE: usize = 36;
pub const SLOTS: usize = 43;
pub const ENDER_SLOTS: usize = 27;
const INFINITE: i32 = -1;
pub const EFFECTS: [(EffectKind, &str); 12] = [
    (EffectKind::Hunger, "minecraft:hunger"),
    (EffectKind::Poison, "minecraft:poison"),
    (EffectKind::Regeneration, "minecraft:regeneration"),
    (EffectKind::Absorption, "minecraft:absorption"),
    (EffectKind::Resistance, "minecraft:resistance"),
    (EffectKind::FireResistance, "minecraft:fire_resistance"),
    (EffectKind::Nausea, "minecraft:nausea"),
    (EffectKind::Slowness, "minecraft:slowness"),
    (EffectKind::Weakness, "minecraft:weakness"),
    (EffectKind::Wither, "minecraft:wither"),
    (EffectKind::Levitation, "minecraft:levitation"),
    (EffectKind::SlowFalling, "minecraft:slow_falling"),
];

struct EffectModifier {
    effect: EffectKind,
    attribute: &'static str,
    base: f64,
    id: &'static str,
    amount: f64,
    operation: &'static str,
}

const MODIFIERS: [EffectModifier; 3] = [
    EffectModifier {
        effect: EffectKind::Slowness,
        attribute: "minecraft:movement_speed",
        base: 0.10000000149011612,
        id: "minecraft:effect.slowness",
        amount: -0.15000000596046448,
        operation: "add_multiplied_total",
    },
    EffectModifier {
        effect: EffectKind::Weakness,
        attribute: "minecraft:attack_damage",
        base: 1.0,
        id: "minecraft:effect.weakness",
        amount: -4.0,
        operation: "add_value",
    },
    EffectModifier {
        effect: EffectKind::Absorption,
        attribute: "minecraft:max_absorption",
        base: 0.0,
        id: "minecraft:effect.absorption",
        amount: 4.0,
        operation: "add_value",
    },
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stats {
    pub status: SurvivalStatus,
    pub fire: i32,
    pub score: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Place {
    pub dimension: Dimension,
    pub feet: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub motion: [f64; 3],
    pub on_ground: bool,
    pub fall_distance: f64,
}

impl Place {
    pub fn standing(dimension: Dimension, feet: [f64; 3], yaw: f32, pitch: f32) -> Self {
        Self {
            dimension,
            feet,
            yaw,
            pitch,
            motion: [0.0; 3],
            on_ground: true,
            fall_distance: 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlobalPos {
    pub dimension: Dimension,
    pub pos: (i32, i32, i32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Respawn {
    pub at: GlobalPos,
    pub yaw: f32,
    pub pitch: f32,
    pub forced: bool,
}

pub const EQUIPMENT: [(usize, &str); 7] = [
    (36, "feet"),
    (37, "legs"),
    (38, "chest"),
    (39, "head"),
    (40, "offhand"),
    (41, "body"),
    (42, "saddle"),
];
const OWNED: [&str; 21] = [
    "playerGameType",
    "previousPlayerGameType",
    "abilities",
    "Inventory",
    "equipment",
    "SelectedItemSlot",
    "EnderItems",
    "Health",
    "AbsorptionAmount",
    "foodLevel",
    "foodTickTimer",
    "foodSaturationLevel",
    "foodExhaustionLevel",
    "XpLevel",
    "XpP",
    "XpTotal",
    "Score",
    "Air",
    "Fire",
    "respawn",
    "LastDeathLocation",
];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerData {
    pub mode: Option<PlayerMode>,
    pub inventory: Option<Vec<Option<ItemStack>>>,
    pub selected: usize,
    pub stats: Option<Stats>,
    pub place: Option<Place>,
    pub uuid: Option<String>,
    pub ender: Option<Vec<Option<ItemStack>>>,
    pub respawn: Option<Respawn>,
    pub last_death: Option<GlobalPos>,
    pub portal_cooldown: Option<i32>,
    pub seen_credits: Option<bool>,
}

impl PlayerData {
    pub fn read(tag: &Tag, recipes: &RecipeBook) -> Self {
        let inventory = tag.get("Inventory").map(|list| {
            let mut slots = slots(list, STORAGE, recipes);
            slots.resize(SLOTS, None);
            for (slot, name) in EQUIPMENT {
                slots[slot] = tag
                    .get("equipment")
                    .and_then(|equipment| equipment.get(name))
                    .and_then(|stack| stack_of(stack, recipes));
            }
            slots
        });
        Self {
            mode: PlayerMode::from_tag(tag),
            inventory,
            selected: tag
                .get("SelectedItemSlot")
                .and_then(Tag::as_i64)
                .and_then(|slot| usize::try_from(slot).ok())
                .filter(|slot| *slot < HOTBAR)
                .unwrap_or(0),
            stats: read_stats(tag),
            place: read_place(tag),
            uuid: tag
                .get("UUID")
                .and_then(Tag::as_ints)
                .and_then(|ints| uuid_from_ints(&ints)),
            ender: tag
                .get("EnderItems")
                .map(|list| slots(list, ENDER_SLOTS, recipes)),
            respawn: tag.get("respawn").and_then(read_respawn),
            last_death: tag.get("LastDeathLocation").and_then(global_pos),
            portal_cooldown: tag
                .get("PortalCooldown")
                .and_then(Tag::as_i64)
                .map(|ticks| ticks.clamp(i32::MIN.into(), i32::MAX.into()) as i32),
            seen_credits: tag
                .get("seenCredits")
                .and_then(Tag::as_i64)
                .map(|seen| seen != 0),
        }
    }

    pub fn entries(&self) -> BTreeMap<String, Tag> {
        let mut entries = BTreeMap::new();
        if let Some(Tag::Compound(mode)) = self.mode.map(|mode| mode.to_tag()) {
            entries.extend(mode);
        }
        if let Some(inventory) = &self.inventory {
            entries.insert(
                "Inventory".to_owned(),
                slot_list(inventory.iter().take(STORAGE)),
            );
            let equipment: BTreeMap<String, Tag> = EQUIPMENT
                .iter()
                .filter_map(|&(slot, name)| {
                    let stack = stack_tag(inventory.get(slot)?.as_ref()?)?;
                    Some((name.to_owned(), stack))
                })
                .collect();
            if !equipment.is_empty() {
                entries.insert("equipment".to_owned(), Tag::Compound(equipment));
            }
        }
        entries.insert(
            "SelectedItemSlot".to_owned(),
            Tag::Int(self.selected as i32),
        );
        if let Some(stats) = &self.stats {
            stats_entries(stats, &mut entries);
        }
        entries.extend(self.effect_entries(None, None));
        if let Some(place) = &self.place {
            place_entries(place, &mut entries);
        }
        if let Some(ints) = self.uuid.as_deref().and_then(uuid_to_ints) {
            entries.insert("UUID".to_owned(), Tag::IntArray(ints.to_vec()));
        }
        if let Some(ender) = &self.ender {
            entries.insert(
                "EnderItems".to_owned(),
                slot_list(ender.iter().take(ENDER_SLOTS)),
            );
        }
        if let Some(respawn) = &self.respawn {
            entries.insert("respawn".to_owned(), respawn_tag(respawn));
        }
        if let Some(death) = &self.last_death {
            entries.insert(
                "LastDeathLocation".to_owned(),
                global_pos_tag(death, Vec::new()),
            );
        }
        if let Some(ticks) = self.portal_cooldown {
            entries.insert("PortalCooldown".to_owned(), Tag::Int(ticks));
        }
        if let Some(seen) = self.seen_credits {
            entries.insert("seenCredits".to_owned(), Tag::Byte(i8::from(seen)));
        }
        entries
    }

    pub fn merged(&self, previous: Option<Tag>) -> Tag {
        let mut merged = match previous {
            Some(Tag::Compound(map)) => map,
            _ => BTreeMap::new(),
        };
        let effects = merged.remove("active_effects");
        let attributes = merged.remove("attributes");
        for key in OWNED {
            merged.remove(key);
        }
        merged.extend(self.entries());
        merged.extend(self.effect_entries(effects.as_ref(), attributes.as_ref()));
        Tag::Compound(merged)
    }

    fn effect_entries(
        &self,
        effects: Option<&Tag>,
        attributes: Option<&Tag>,
    ) -> Vec<(String, Tag)> {
        let status = self.stats.as_ref().map(|stats| &stats.status);
        let mut list: Vec<Tag> = effects
            .and_then(Tag::as_list)
            .unwrap_or(&[])
            .iter()
            .filter(|effect| effect_kind(effect).is_none())
            .cloned()
            .collect();
        list.extend(status.map_or_else(Vec::new, effect_tags));
        let mut entries = Vec::new();
        if !list.is_empty() {
            entries.push(("active_effects".to_owned(), Tag::List(list)));
        }
        if let Some(attributes) = attributes_with(attributes, status) {
            entries.push(("attributes".to_owned(), attributes));
        }
        entries
    }
}

fn compound(entries: Vec<(&str, Tag)>) -> Tag {
    Tag::Compound(
        entries
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn stats_entries(stats: &Stats, entries: &mut BTreeMap<String, Tag>) {
    let status = &stats.status;
    let whole = |value: u32| Tag::Int(i32::try_from(value).unwrap_or(i32::MAX));
    let short = |value: i32| Tag::Short(value.clamp(i16::MIN.into(), i16::MAX.into()) as i16);
    let values = [
        ("Health", Tag::Float(status.health)),
        ("AbsorptionAmount", Tag::Float(status.absorption)),
        ("foodLevel", Tag::Int(i32::from(status.food.level))),
        ("foodTickTimer", whole(status.food.tick_timer())),
        ("foodSaturationLevel", Tag::Float(status.food.saturation)),
        ("foodExhaustionLevel", Tag::Float(status.food.exhaustion)),
        ("XpLevel", whole(status.experience_level)),
        ("XpP", Tag::Float(status.experience_progress)),
        ("XpTotal", whole(status.total_experience)),
        ("Score", Tag::Int(stats.score)),
        ("Air", short(status.air)),
        ("Fire", short(stats.fire)),
    ];
    for (key, value) in values {
        entries.insert(key.to_owned(), value);
    }
}

fn global_pos_tag(at: &GlobalPos, extra: Vec<(&str, Tag)>) -> Tag {
    let (x, y, z) = at.pos;
    let mut entries = vec![
        (
            "dimension",
            Tag::String(at.dimension.dimension_type().to_owned()),
        ),
        ("pos", Tag::IntArray(vec![x, y, z])),
    ];
    entries.extend(extra);
    compound(entries)
}

fn global_pos(tag: &Tag) -> Option<GlobalPos> {
    let dimension = tag.get("dimension")?.as_str()?;
    let [x, y, z] = tag.get("pos")?.as_ints()?[..] else {
        return None;
    };
    Some(GlobalPos {
        dimension: Dimension::parse(dimension).unwrap_or(Dimension::Overworld),
        pos: (x, y, z),
    })
}

fn respawn_tag(respawn: &Respawn) -> Tag {
    let mut extra = vec![
        ("yaw", Tag::Float(respawn.yaw)),
        ("pitch", Tag::Float(respawn.pitch)),
    ];
    if respawn.forced {
        extra.push(("forced", Tag::Byte(1)));
    }
    global_pos_tag(&respawn.at, extra)
}

fn read_respawn(tag: &Tag) -> Option<Respawn> {
    let angle = |key: &str| tag.get(key).and_then(Tag::as_f64).unwrap_or(0.0) as f32;
    Some(Respawn {
        at: global_pos(tag)?,
        yaw: angle("yaw"),
        pitch: angle("pitch"),
        forced: tag
            .get("forced")
            .and_then(Tag::as_i64)
            .is_some_and(|forced| forced != 0),
    })
}

fn place_entries(place: &Place, entries: &mut BTreeMap<String, Tag>) {
    let doubles = |values: [f64; 3]| Tag::List(values.into_iter().map(Tag::Double).collect());
    let values = [
        ("Pos", doubles(place.feet)),
        ("Motion", doubles(place.motion)),
        (
            "Rotation",
            Tag::List(vec![Tag::Float(place.yaw), Tag::Float(place.pitch)]),
        ),
        ("OnGround", Tag::Byte(i8::from(place.on_ground))),
        ("fall_distance", Tag::Double(place.fall_distance)),
        (
            "Dimension",
            Tag::String(place.dimension.dimension_type().to_owned()),
        ),
    ];
    for (key, value) in values {
        entries.insert(key.to_owned(), value);
    }
}

fn read_place(tag: &Tag) -> Option<Place> {
    let feet = triple(tag.get("Pos")?).filter(|feet| feet.iter().all(|v| v.is_finite()))?;
    let rotation = tag.get("Rotation").and_then(Tag::as_list).unwrap_or(&[]);
    let angle = |index: usize| rotation.get(index).and_then(Tag::as_f64).unwrap_or(0.0) as f32;
    Some(Place {
        dimension: tag
            .get("Dimension")
            .and_then(Tag::as_str)
            .and_then(Dimension::parse)
            .unwrap_or(Dimension::Overworld),
        feet,
        yaw: angle(0),
        pitch: angle(1),
        motion: tag.get("Motion").and_then(triple).unwrap_or([0.0; 3]),
        on_ground: tag
            .get("OnGround")
            .and_then(Tag::as_i64)
            .is_some_and(|on| on != 0),
        fall_distance: tag
            .get("fall_distance")
            .and_then(Tag::as_f64)
            .unwrap_or(0.0),
    })
}

fn triple(tag: &Tag) -> Option<[f64; 3]> {
    match tag.as_list()? {
        [x, y, z] => Some([x.as_f64()?, y.as_f64()?, z.as_f64()?]),
        _ => None,
    }
}

fn read_stats(tag: &Tag) -> Option<Stats> {
    let health = tag.get("Health")?.as_f64()? as f32;
    let float = |key: &str, default: f32| {
        tag.get(key)
            .and_then(Tag::as_f64)
            .map_or(default, |v| v as f32)
    };
    let int = |key: &str, default: i64| tag.get(key).and_then(Tag::as_i64).unwrap_or(default);
    let whole = |key: &str| int(key, 0).clamp(0, i64::from(i32::MAX)) as u32;
    let mut status = SurvivalStatus::default();
    for effect in tag
        .get("active_effects")
        .and_then(Tag::as_list)
        .unwrap_or(&[])
    {
        let Some(kind) = effect_kind(effect) else {
            continue;
        };
        let amplifier = effect.get("amplifier").and_then(Tag::as_i64).unwrap_or(0) as u8;
        let duration = match effect.get("duration").and_then(Tag::as_i64).unwrap_or(0) {
            -1 => u32::MAX,
            ticks if ticks > 0 => ticks.min(i64::from(i32::MAX)) as u32,
            _ => continue,
        };
        status.add_effect(kind, duration, amplifier);
    }
    status.health = health.min(status.max_health);
    status.absorption = float("AbsorptionAmount", 0.0).max(0.0);
    status.food.level = int("foodLevel", 20).clamp(0, 20) as u8;
    status.food.set_tick_timer(whole("foodTickTimer"));
    status.food.saturation = float("foodSaturationLevel", 5.0);
    status.food.exhaustion = float("foodExhaustionLevel", 0.0);
    status.experience_level = whole("XpLevel");
    status.experience_progress = float("XpP", 0.0);
    status.total_experience = whole("XpTotal");
    status.air = int("Air", i64::from(status.max_air)) as i32;
    Some(Stats {
        status,
        fire: int("Fire", 0) as i32,
        score: int("Score", 0) as i32,
    })
}

fn effect_kind(effect: &Tag) -> Option<EffectKind> {
    let id = effect.get("id")?.as_str()?;
    EFFECTS
        .iter()
        .find(|(_, known)| *known == id)
        .map(|(kind, _)| *kind)
}

fn effect_tags(status: &SurvivalStatus) -> Vec<Tag> {
    EFFECTS
        .iter()
        .filter_map(|&(kind, id)| {
            let effect = status.effect(kind)?;
            Some(compound(vec![
                ("id", Tag::String(id.to_owned())),
                ("amplifier", Tag::Byte(effect.amplifier as i8)),
                (
                    "duration",
                    Tag::Int(i32::try_from(effect.duration).unwrap_or(INFINITE)),
                ),
                ("ambient", Tag::Byte(0)),
                ("show_particles", Tag::Byte(1)),
                ("show_icon", Tag::Byte(1)),
            ]))
        })
        .collect()
}

fn attributes_with(previous: Option<&Tag>, status: Option<&SurvivalStatus>) -> Option<Tag> {
    let mut list: Vec<Tag> = previous
        .and_then(Tag::as_list)
        .map_or_else(Vec::new, <[Tag]>::to_vec);
    for entry in &mut list {
        if let Tag::Compound(map) = entry
            && let Some(Tag::List(modifiers)) = map.get_mut("modifiers")
        {
            modifiers.retain(|modifier| {
                let id = modifier.get("id").and_then(Tag::as_str);
                !MODIFIERS.iter().any(|known| Some(known.id) == id)
            });
            if modifiers.is_empty() {
                map.remove("modifiers");
            }
        }
    }
    for known in &MODIFIERS {
        let Some(effect) = status.and_then(|status| status.effect(known.effect)) else {
            continue;
        };
        let modifier = compound(vec![
            ("id", Tag::String(known.id.to_owned())),
            (
                "amount",
                Tag::Double(known.amount * (f64::from(effect.amplifier) + 1.0)),
            ),
            ("operation", Tag::String(known.operation.to_owned())),
        ]);
        let entry = list
            .iter_mut()
            .find(|entry| entry.get("id").and_then(Tag::as_str) == Some(known.attribute));
        match entry {
            Some(Tag::Compound(map)) => match map.get_mut("modifiers") {
                Some(Tag::List(modifiers)) => modifiers.push(modifier),
                _ => {
                    map.insert("modifiers".to_owned(), Tag::List(vec![modifier]));
                }
            },
            _ => list.push(compound(vec![
                ("id", Tag::String(known.attribute.to_owned())),
                ("base", Tag::Double(known.base)),
                ("modifiers", Tag::List(vec![modifier])),
            ])),
        }
    }
    (!list.is_empty()).then_some(Tag::List(list))
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::game_mode::GameMode;
    use serde_json::json;

    fn stack(id: &str, count: u8) -> ItemStack {
        ItemStack::new(id, count)
    }

    fn filled() -> Vec<Option<ItemStack>> {
        let mut slots = vec![None; SLOTS];
        slots[0] = Some(stack("minecraft:cobblestone", 64));
        let mut sword = stack("minecraft:diamond_sword", 1);
        sword.max = 1;
        sword.components = Some(json!({
            "minecraft:enchantments": {"minecraft:sharpness": 5},
            "minecraft:damage": 12
        }));
        slots[8] = Some(sword);
        slots[35] = Some(stack("minecraft:torch", 3));
        slots[36] = Some(stack("minecraft:diamond_boots", 1));
        slots[37] = Some(stack("minecraft:diamond_leggings", 1));
        slots[38] = Some(stack("minecraft:diamond_chestplate", 1));
        slots[39] = Some(stack("minecraft:diamond_helmet", 1));
        slots[40] = Some(stack("minecraft:shield", 1));
        slots
    }

    #[test]
    fn storage_slots_go_to_inventory_with_a_slot_byte_and_armour_to_equipment() {
        let data = PlayerData {
            inventory: Some(filled()),
            selected: 4,
            ..PlayerData::default()
        };
        let entries = data.entries();
        let inventory = entries["Inventory"].as_list().unwrap();
        assert_eq!(inventory.len(), 3);
        assert_eq!(
            inventory[0],
            compound(vec![
                ("Slot", Tag::Byte(0)),
                ("count", Tag::Int(64)),
                ("id", Tag::String("minecraft:cobblestone".to_owned())),
            ])
        );
        assert_eq!(inventory[1].get("Slot"), Some(&Tag::Byte(8)));
        assert_eq!(
            inventory[1]
                .get("components")
                .and_then(|c| c.get("minecraft:enchantments"))
                .and_then(|e| e.get("minecraft:sharpness")),
            Some(&Tag::Int(5))
        );
        assert_eq!(inventory[2].get("Slot"), Some(&Tag::Byte(35)));
        let equipment = entries["equipment"].as_compound().unwrap();
        let names: Vec<&str> = equipment.keys().map(String::as_str).collect();
        assert_eq!(names, ["chest", "feet", "head", "legs", "offhand"]);
        assert_eq!(
            equipment["head"].get("id"),
            Some(&Tag::String("minecraft:diamond_helmet".to_owned()))
        );
        assert_eq!(equipment["offhand"].get("count"), Some(&Tag::Int(1)));
        assert_eq!(entries["SelectedItemSlot"], Tag::Int(4));
    }

    #[test]
    fn the_inventory_comes_back_from_its_file_in_the_same_slots() {
        let mut mode = PlayerMode::new(GameMode::Creative);
        mode.abilities.flying = true;
        let mut slots = filled();
        slots[41] = Some(stack("minecraft:diamond_horse_armor", 1));
        slots[42] = Some(stack("minecraft:saddle", 1));
        let data = PlayerData {
            mode: Some(mode),
            inventory: Some(slots),
            selected: 8,
            ..PlayerData::default()
        };
        let tag = data.merged(None);
        let recipes = RecipeBook::default();
        let read = PlayerData::read(&tag, &recipes);
        assert_eq!(read.mode, Some(mode));
        assert_eq!(read.selected, 8);
        let restored = read.inventory.unwrap();
        assert_eq!(restored.len(), SLOTS);
        for (slot, (had, back)) in data.inventory.unwrap().iter().zip(&restored).enumerate() {
            assert_eq!(
                had.as_ref().map(|s| (&s.id, s.count, &s.components)),
                back.as_ref().map(|s| (&s.id, s.count, &s.components)),
                "slot {slot}"
            );
        }
        assert_eq!(
            tag.get("equipment")
                .and_then(|e| e.get("saddle"))
                .and_then(|s| s.get("id")),
            Some(&Tag::String("minecraft:saddle".to_owned()))
        );
        assert_eq!(
            tag.get("equipment")
                .and_then(|e| e.get("body"))
                .and_then(|s| s.get("id")),
            Some(&Tag::String("minecraft:diamond_horse_armor".to_owned()))
        );
    }

    #[test]
    fn guns_and_empty_stacks_are_not_saved() {
        let mut slots = vec![None; SLOTS];
        slots[0] = Some(crate::guns::weapon_item(7));
        slots[1] = Some(stack("minecraft:dirt", 0));
        slots[2] = Some(stack("minecraft:dirt", 2));
        let entries = PlayerData {
            inventory: Some(slots),
            ..PlayerData::default()
        }
        .entries();
        let inventory = entries["Inventory"].as_list().unwrap();
        assert_eq!(inventory.len(), 1);
        assert_eq!(inventory[0].get("Slot"), Some(&Tag::Byte(2)));
        assert!(!entries.contains_key("equipment"));
    }

    #[test]
    fn an_empty_inventory_is_still_written_and_read_back_as_present() {
        let data = PlayerData {
            inventory: Some(vec![None; SLOTS]),
            ..PlayerData::default()
        };
        let tag = data.merged(None);
        assert_eq!(tag.get("Inventory"), Some(&Tag::List(Vec::new())));
        let read = PlayerData::read(&tag, &RecipeBook::default());
        assert_eq!(read.inventory, Some(vec![None; SLOTS]));
        let missing = PlayerData::read(&compound(Vec::new()), &RecipeBook::default());
        assert_eq!(missing.inventory, None);
        assert_eq!(missing.mode, None);
        assert_eq!(missing.selected, 0);
    }

    #[test]
    fn merging_keeps_unknown_keys_and_drops_armour_that_was_taken_off() {
        let previous = compound(vec![
            (
                "recipeBook",
                compound(vec![("recipes", Tag::List(Vec::new()))]),
            ),
            ("seenCredits", Tag::Byte(1)),
            (
                "equipment",
                compound(vec![(
                    "head",
                    compound(vec![
                        ("id", Tag::String("minecraft:iron_helmet".to_owned())),
                        ("count", Tag::Int(1)),
                    ]),
                )]),
            ),
            ("SelectedItemSlot", Tag::Int(3)),
        ]);
        let data = PlayerData {
            mode: Some(PlayerMode::new(GameMode::Survival)),
            inventory: Some(vec![None; SLOTS]),
            selected: 1,
            ..PlayerData::default()
        };
        let merged = data.merged(Some(previous.clone()));
        assert_eq!(merged.get("recipeBook"), previous.get("recipeBook"));
        assert_eq!(merged.get("seenCredits"), Some(&Tag::Byte(1)));
        assert_eq!(merged.get("equipment"), None);
        assert_eq!(merged.get("SelectedItemSlot"), Some(&Tag::Int(1)));
        assert_eq!(merged.get("playerGameType"), Some(&Tag::Int(0)));
        assert_eq!(
            merged.get("abilities").and_then(|a| a.get("mayfly")),
            Some(&Tag::Byte(0))
        );
    }

    #[test]
    fn slots_read_as_unsigned_bytes_and_bad_entries_are_skipped() {
        let list = Tag::List(vec![
            compound(vec![
                ("Slot", Tag::Byte(-1)),
                ("id", Tag::String("minecraft:stone".to_owned())),
            ]),
            compound(vec![
                ("Slot", Tag::Byte(5)),
                ("id", Tag::String("minecraft:stone".to_owned())),
            ]),
            compound(vec![("Slot", Tag::Byte(6)), ("count", Tag::Int(2))]),
            compound(vec![
                ("Slot", Tag::Byte(7)),
                ("id", Tag::String("minecraft:stone".to_owned())),
                ("count", Tag::Int(0)),
            ]),
            compound(vec![("id", Tag::String("minecraft:stone".to_owned()))]),
        ]);
        let read = slots(&list, STORAGE, &RecipeBook::default());
        assert_eq!(read.iter().flatten().count(), 1);
        assert_eq!(
            read[5].as_ref().map(|s| (s.id.as_str(), s.count)),
            Some(("minecraft:stone", 1))
        );
        let list = Tag::List(vec![compound(vec![
            ("Slot", Tag::Byte(-56)),
            ("id", Tag::String("minecraft:stone".to_owned())),
        ])]);
        assert!(slots(&list, 256, &RecipeBook::default())[200].is_some());
    }

    #[test]
    fn counts_are_ints_capped_at_vanilla_s_limit_and_old_string_components_load() {
        let tag = stack_tag(&stack("minecraft:stone", 120)).unwrap();
        assert_eq!(tag.get("count"), Some(&Tag::Int(99)));
        let old = compound(vec![
            ("id", Tag::String("minecraft:iron_sword".to_owned())),
            (
                "components",
                Tag::String(r#"{"minecraft:damage":3}"#.to_owned()),
            ),
        ]);
        let read = stack_of(&old, &RecipeBook::default()).unwrap();
        assert_eq!(read.count, 1);
        assert_eq!(read.components, Some(json!({"minecraft:damage": 3})));
        let written = stack_tag(&read).unwrap();
        assert_eq!(
            written.get("components"),
            Some(&compound(vec![("minecraft:damage", Tag::Int(3))]))
        );
    }

    fn hurt_and_fed() -> Stats {
        let mut status = SurvivalStatus::default();
        status.health = 13.5;
        status.food.level = 17;
        status.food.saturation = 2.25;
        status.food.exhaustion = 1.5;
        status.food.set_tick_timer(42);
        status.experience_level = 7;
        status.experience_progress = 0.25;
        status.total_experience = 120;
        status.air = 140;
        Stats {
            status,
            fire: 35,
            score: 120,
        }
    }

    fn with_stats(stats: Stats) -> PlayerData {
        PlayerData {
            stats: Some(stats),
            ..PlayerData::default()
        }
    }

    #[test]
    fn health_hunger_and_experience_use_vanilla_keys_and_tag_types() {
        let entries = with_stats(hurt_and_fed()).entries();
        let expected = [
            ("Health", Tag::Float(13.5)),
            ("AbsorptionAmount", Tag::Float(0.0)),
            ("foodLevel", Tag::Int(17)),
            ("foodTickTimer", Tag::Int(42)),
            ("foodSaturationLevel", Tag::Float(2.25)),
            ("foodExhaustionLevel", Tag::Float(1.5)),
            ("XpLevel", Tag::Int(7)),
            ("XpP", Tag::Float(0.25)),
            ("XpTotal", Tag::Int(120)),
            ("Score", Tag::Int(120)),
            ("Air", Tag::Short(140)),
            ("Fire", Tag::Short(35)),
        ];
        for (key, tag) in expected {
            assert_eq!(entries.get(key), Some(&tag), "{key}");
        }
        assert!(!entries.contains_key("active_effects"));
        assert!(!entries.contains_key("attributes"));
        let no_stats = PlayerData::default().entries();
        assert!(!no_stats.contains_key("Health"));
        assert!(!no_stats.contains_key("foodLevel"));
    }

    #[test]
    fn stats_round_trip_and_absorption_is_restored_after_the_effects() {
        let mut stats = hurt_and_fed();
        stats.fire = -20;
        stats.status.add_effect(EffectKind::Absorption, 1200, 1);
        stats
            .status
            .add_effect(EffectKind::Regeneration, u32::MAX, 1);
        stats.status.add_effect(EffectKind::Poison, 300, 0);
        stats.status.absorption = 3.0;
        let tag = with_stats(stats).merged(None);
        let read = PlayerData::read(&tag, &RecipeBook::default())
            .stats
            .unwrap();
        assert_eq!(read, stats);
        assert_eq!(read.status.absorption, 3.0);
        assert_eq!(
            read.status
                .effect(EffectKind::Regeneration)
                .map(|e| e.duration),
            Some(u32::MAX)
        );
    }

    #[test]
    fn effects_are_saved_like_mob_effect_instance_with_infinite_as_minus_one() {
        let mut stats = hurt_and_fed();
        stats
            .status
            .add_effect(EffectKind::Regeneration, u32::MAX, 1);
        stats.status.add_effect(EffectKind::Weakness, 600, 0);
        let entries = with_stats(stats).entries();
        let effects = entries["active_effects"].as_list().unwrap();
        assert_eq!(
            effects[0],
            compound(vec![
                ("ambient", Tag::Byte(0)),
                ("amplifier", Tag::Byte(1)),
                ("duration", Tag::Int(-1)),
                ("id", Tag::String("minecraft:regeneration".to_owned())),
                ("show_icon", Tag::Byte(1)),
                ("show_particles", Tag::Byte(1)),
            ])
        );
        assert_eq!(
            effects[1].get("id"),
            Some(&Tag::String("minecraft:weakness".to_owned()))
        );
        assert_eq!(effects[1].get("duration"), Some(&Tag::Int(600)));
    }

    #[test]
    fn levitation_and_slow_falling_are_saved_and_read_back() {
        let mut stats = hurt_and_fed();
        stats.status.add_effect(EffectKind::Levitation, 200, 1);
        stats.status.add_effect(EffectKind::SlowFalling, 4800, 0);
        let data = with_stats(stats);
        let entries = data.entries();
        let ids: Vec<&str> = entries["active_effects"]
            .as_list()
            .unwrap()
            .iter()
            .filter_map(|e| e.get("id").and_then(Tag::as_str))
            .collect();
        assert_eq!(ids, vec!["minecraft:levitation", "minecraft:slow_falling"]);
        let read = PlayerData::read(&data.merged(None), &RecipeBook::default());
        let status = read.stats.unwrap().status;
        assert_eq!(
            status
                .effect(EffectKind::Levitation)
                .map(|e| (e.duration, e.amplifier)),
            Some((200, 1))
        );
        assert_eq!(
            status.effect(EffectKind::SlowFalling).map(|e| e.duration),
            Some(4800)
        );
    }

    #[test]
    fn effects_this_game_does_not_simulate_survive_and_ended_ones_are_removed() {
        let effect = |id: &str| {
            compound(vec![
                ("id", Tag::String(id.to_owned())),
                ("amplifier", Tag::Byte(0)),
                ("duration", Tag::Int(400)),
            ])
        };
        let previous = compound(vec![(
            "active_effects",
            Tag::List(vec![effect("minecraft:speed"), effect("minecraft:poison")]),
        )]);
        let merged = with_stats(hurt_and_fed()).merged(Some(previous.clone()));
        assert_eq!(
            merged.get("active_effects"),
            Some(&Tag::List(vec![effect("minecraft:speed")]))
        );
        let read = PlayerData::read(&previous, &RecipeBook::default());
        assert_eq!(read.stats, None);
        let only_ours = compound(vec![(
            "active_effects",
            Tag::List(vec![effect("minecraft:poison")]),
        )]);
        let merged = with_stats(hurt_and_fed()).merged(Some(only_ours));
        assert_eq!(merged.get("active_effects"), None);
    }

    #[test]
    fn effect_attribute_modifiers_follow_the_effects() {
        let modifier = |id: &str, amount: f64, operation: &str| {
            compound(vec![
                ("id", Tag::String(id.to_owned())),
                ("amount", Tag::Double(amount)),
                ("operation", Tag::String(operation.to_owned())),
            ])
        };
        let sprinting = modifier("minecraft:sprinting", 0.3, "add_multiplied_total");
        let previous = compound(vec![(
            "attributes",
            Tag::List(vec![
                compound(vec![
                    ("id", Tag::String("minecraft:movement_speed".to_owned())),
                    ("base", Tag::Double(0.10000000149011612)),
                    (
                        "modifiers",
                        Tag::List(vec![
                            modifier("minecraft:effect.slowness", -0.3, "add_multiplied_total"),
                            sprinting.clone(),
                        ]),
                    ),
                ]),
                compound(vec![
                    ("id", Tag::String("minecraft:attack_damage".to_owned())),
                    ("base", Tag::Double(1.0)),
                    (
                        "modifiers",
                        Tag::List(vec![modifier(
                            "minecraft:effect.weakness",
                            -4.0,
                            "add_value",
                        )]),
                    ),
                ]),
            ]),
        )]);
        let mut stats = hurt_and_fed();
        stats.status.add_effect(EffectKind::Absorption, 2400, 1);
        let merged = with_stats(stats).merged(Some(previous));
        let attributes = merged.get("attributes").and_then(Tag::as_list).unwrap();
        assert_eq!(attributes.len(), 3);
        assert_eq!(
            attributes[0].get("modifiers"),
            Some(&Tag::List(vec![sprinting]))
        );
        assert_eq!(attributes[1].get("modifiers"), None);
        assert_eq!(attributes[1].get("base"), Some(&Tag::Double(1.0)));
        assert_eq!(
            attributes[2],
            compound(vec![
                ("id", Tag::String("minecraft:max_absorption".to_owned())),
                ("base", Tag::Double(0.0)),
                (
                    "modifiers",
                    Tag::List(vec![modifier(
                        "minecraft:effect.absorption",
                        8.0,
                        "add_value"
                    )]),
                ),
            ])
        );
        let mut slow = hurt_and_fed();
        slow.status.add_effect(EffectKind::Slowness, 100, 2);
        let fresh = with_stats(slow).entries();
        let speed = &fresh["attributes"].as_list().unwrap()[0];
        assert_eq!(
            speed.get("modifiers").and_then(Tag::as_list).unwrap()[0].get("amount"),
            Some(&Tag::Double(-0.15000000596046448 * 3.0))
        );
        assert_eq!(speed.get("base"), Some(&Tag::Double(0.10000000149011612)));
    }

    #[test]
    fn missing_food_and_air_keys_take_vanilla_defaults() {
        let tag = compound(vec![("Health", Tag::Float(30.0))]);
        let stats = PlayerData::read(&tag, &RecipeBook::default())
            .stats
            .unwrap();
        assert_eq!(stats.status.health, 20.0);
        assert_eq!(stats.status.food.level, 20);
        assert_eq!(stats.status.food.saturation, 5.0);
        assert_eq!(stats.status.food.tick_timer(), 0);
        assert_eq!(stats.status.air, 300);
        assert_eq!((stats.fire, stats.score), (0, 0));
        assert_eq!(stats.status.experience_level, 0);
    }

    fn up_a_hill() -> Place {
        Place {
            dimension: Dimension::Overworld,
            feet: [100.5, 72.0, -33.25],
            yaw: -135.0,
            pitch: 20.5,
            motion: [0.0, -0.0784000015258789, 0.0],
            on_ground: true,
            fall_distance: 0.0,
        }
    }

    #[test]
    fn the_place_is_saved_as_entity_pos_rotation_and_dimension() {
        let uuid = "12345678-9abc-def0-1234-56789abcdef0".to_owned();
        let entries = PlayerData {
            place: Some(Place {
                fall_distance: 2.5,
                on_ground: false,
                ..up_a_hill()
            }),
            uuid: Some(uuid.clone()),
            ..PlayerData::default()
        }
        .entries();
        assert_eq!(
            entries["Pos"],
            Tag::List(vec![
                Tag::Double(100.5),
                Tag::Double(72.0),
                Tag::Double(-33.25)
            ])
        );
        assert_eq!(
            entries["Rotation"],
            Tag::List(vec![Tag::Float(-135.0), Tag::Float(20.5)])
        );
        assert_eq!(
            entries["Motion"],
            Tag::List(vec![
                Tag::Double(0.0),
                Tag::Double(-0.0784000015258789),
                Tag::Double(0.0)
            ])
        );
        assert_eq!(entries["OnGround"], Tag::Byte(0));
        assert_eq!(entries["fall_distance"], Tag::Double(2.5));
        assert_eq!(
            entries["Dimension"],
            Tag::String("minecraft:overworld".to_owned())
        );
        assert_eq!(
            entries["UUID"],
            Tag::IntArray(uuid_to_ints(&uuid).unwrap().to_vec())
        );
        let read = PlayerData::read(&Tag::Compound(entries), &RecipeBook::default());
        assert_eq!(read.uuid, Some(uuid));
        assert_eq!(read.place.map(|place| place.fall_distance), Some(2.5));
    }

    #[test]
    fn a_place_round_trips_and_an_unknown_place_keeps_the_file_s_position() {
        let place = up_a_hill();
        let tag = PlayerData {
            place: Some(place),
            ..PlayerData::default()
        }
        .merged(None);
        assert_eq!(
            PlayerData::read(&tag, &RecipeBook::default()).place,
            Some(place)
        );
        let nether = compound(vec![
            (
                "Pos",
                Tag::List(vec![Tag::Double(1.0), Tag::Double(40.0), Tag::Double(2.0)]),
            ),
            ("Dimension", Tag::String("minecraft:the_nether".to_owned())),
        ]);
        let kept = PlayerData::default().merged(Some(nether.clone()));
        assert_eq!(kept.get("Pos"), nether.get("Pos"));
        assert_eq!(kept.get("Dimension"), nether.get("Dimension"));
        let read = PlayerData::read(&nether, &RecipeBook::default())
            .place
            .unwrap();
        assert_eq!(read.dimension, Dimension::Nether);
        assert_eq!((read.yaw, read.pitch, read.on_ground), (0.0, 0.0, false));
        assert_eq!(read.motion, [0.0; 3]);
    }

    #[test]
    fn a_broken_or_unknown_position_reads_safely() {
        let short = compound(vec![(
            "Pos",
            Tag::List(vec![Tag::Double(1.0), Tag::Double(2.0)]),
        )]);
        assert_eq!(PlayerData::read(&short, &RecipeBook::default()).place, None);
        let nan = compound(vec![(
            "Pos",
            Tag::List(vec![
                Tag::Double(f64::NAN),
                Tag::Double(2.0),
                Tag::Double(3.0),
            ]),
        )]);
        assert_eq!(PlayerData::read(&nan, &RecipeBook::default()).place, None);
        let moon = compound(vec![
            (
                "Pos",
                Tag::List(vec![Tag::Double(1.0), Tag::Double(2.0), Tag::Double(3.0)]),
            ),
            ("Dimension", Tag::String("minecraft:the_moon".to_owned())),
        ]);
        let read = PlayerData::read(&moon, &RecipeBook::default())
            .place
            .unwrap();
        assert_eq!(read.dimension, Dimension::Overworld);
        assert_eq!(read.feet, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn the_ender_chest_is_saved_as_ender_items_with_slot_bytes() {
        let mut ender = vec![None; ENDER_SLOTS];
        ender[0] = Some(stack("minecraft:diamond", 5));
        ender[26] = Some(stack("minecraft:ender_pearl", 16));
        let data = PlayerData {
            ender: Some(ender.clone()),
            ..PlayerData::default()
        };
        let tag = data.merged(None);
        let items = tag.get("EnderItems").and_then(Tag::as_list).unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].get("Slot"), Some(&Tag::Byte(0)));
        assert_eq!(items[1].get("Slot"), Some(&Tag::Byte(26)));
        assert_eq!(items[1].get("count"), Some(&Tag::Int(16)));
        let read = PlayerData::read(&tag, &RecipeBook::default());
        let back: Vec<Option<(String, u8)>> = read
            .ender
            .unwrap()
            .into_iter()
            .map(|cell| cell.map(|s| (s.id, s.count)))
            .collect();
        let had: Vec<Option<(String, u8)>> = ender
            .into_iter()
            .map(|cell| cell.map(|s| (s.id, s.count)))
            .collect();
        assert_eq!(back, had);
        let emptied = PlayerData {
            ender: Some(vec![None; ENDER_SLOTS]),
            ..PlayerData::default()
        }
        .merged(Some(tag));
        assert_eq!(emptied.get("EnderItems"), Some(&Tag::List(Vec::new())));
        assert_eq!(
            PlayerData::read(&compound(Vec::new()), &RecipeBook::default()).ender,
            None
        );
    }

    fn bed() -> Respawn {
        Respawn {
            at: GlobalPos {
                dimension: Dimension::Overworld,
                pos: (-12, 70, 340),
            },
            yaw: 90.0,
            pitch: 0.0,
            forced: false,
        }
    }

    #[test]
    fn the_respawn_point_and_last_death_use_vanilla_s_layout() {
        let point = Respawn {
            forced: true,
            pitch: 12.5,
            ..bed()
        };
        let death = GlobalPos {
            dimension: Dimension::Overworld,
            pos: (5, -40, 7),
        };
        let entries = PlayerData {
            respawn: Some(point),
            last_death: Some(death),
            ..PlayerData::default()
        }
        .entries();
        assert_eq!(
            entries["respawn"],
            compound(vec![
                ("dimension", Tag::String("minecraft:overworld".to_owned())),
                ("pos", Tag::IntArray(vec![-12, 70, 340])),
                ("yaw", Tag::Float(90.0)),
                ("pitch", Tag::Float(12.5)),
                ("forced", Tag::Byte(1)),
            ])
        );
        assert_eq!(
            entries["LastDeathLocation"],
            compound(vec![
                ("dimension", Tag::String("minecraft:overworld".to_owned())),
                ("pos", Tag::IntArray(vec![5, -40, 7])),
            ])
        );
        let read = PlayerData::read(&Tag::Compound(entries), &RecipeBook::default());
        assert_eq!(read.respawn, Some(point));
        assert_eq!(read.last_death, Some(death));
        let unforced = PlayerData {
            respawn: Some(bed()),
            ..PlayerData::default()
        }
        .entries();
        assert_eq!(unforced["respawn"].get("forced"), None);
    }

    #[test]
    fn a_cleared_respawn_point_and_death_leave_the_file() {
        let previous = PlayerData {
            respawn: Some(bed()),
            last_death: Some(bed().at),
            ..PlayerData::default()
        }
        .merged(None);
        assert!(previous.get("respawn").is_some());
        let cleared = PlayerData::default().merged(Some(previous));
        assert_eq!(cleared.get("respawn"), None);
        assert_eq!(cleared.get("LastDeathLocation"), None);
        let without_dimension = compound(vec![(
            "respawn",
            compound(vec![("pos", Tag::IntArray(vec![1, 2, 3]))]),
        )]);
        assert_eq!(
            PlayerData::read(&without_dimension, &RecipeBook::default()).respawn,
            None
        );
        let nether = compound(vec![(
            "LastDeathLocation",
            compound(vec![
                ("dimension", Tag::String("minecraft:the_nether".to_owned())),
                ("pos", Tag::IntArray(vec![1, 2, 3])),
            ]),
        )]);
        assert_eq!(
            PlayerData::read(&nether, &RecipeBook::default()).last_death,
            Some(GlobalPos {
                dimension: Dimension::Nether,
                pos: (1, 2, 3)
            })
        );
    }

    #[test]
    fn the_portal_cooldown_and_seen_credits_are_an_int_and_a_byte_like_vanilla() {
        let entries = PlayerData {
            portal_cooldown: Some(300),
            seen_credits: Some(true),
            ..PlayerData::default()
        }
        .entries();
        assert_eq!(entries["PortalCooldown"], Tag::Int(300));
        assert_eq!(entries["seenCredits"], Tag::Byte(1));
        let read = PlayerData::read(&Tag::Compound(entries), &RecipeBook::default());
        assert_eq!(
            (read.portal_cooldown, read.seen_credits),
            (Some(300), Some(true))
        );
        let unseen = PlayerData {
            portal_cooldown: Some(0),
            seen_credits: Some(false),
            ..PlayerData::default()
        }
        .merged(Some(compound(vec![
            ("PortalCooldown", Tag::Int(12)),
            ("seenCredits", Tag::Byte(1)),
        ])));
        assert_eq!(unseen.get("PortalCooldown"), Some(&Tag::Int(0)));
        assert_eq!(unseen.get("seenCredits"), Some(&Tag::Byte(0)));
        let missing = PlayerData::read(&compound(Vec::new()), &RecipeBook::default());
        assert_eq!(
            (missing.portal_cooldown, missing.seen_credits),
            (None, None)
        );
        let kept = PlayerData::default().merged(Some(unseen));
        assert_eq!(kept.get("PortalCooldown"), Some(&Tag::Int(0)));
        assert_eq!(kept.get("seenCredits"), Some(&Tag::Byte(0)));
        let short = compound(vec![("PortalCooldown", Tag::Short(40))]);
        assert_eq!(
            PlayerData::read(&short, &RecipeBook::default()).portal_cooldown,
            Some(40)
        );
    }

    #[test]
    fn a_selected_slot_outside_the_hotbar_reads_as_the_first() {
        let tag = compound(vec![("SelectedItemSlot", Tag::Int(12))]);
        assert_eq!(PlayerData::read(&tag, &RecipeBook::default()).selected, 0);
        let tag = compound(vec![("SelectedItemSlot", Tag::Int(-2))]);
        assert_eq!(PlayerData::read(&tag, &RecipeBook::default()).selected, 0);
    }
}
