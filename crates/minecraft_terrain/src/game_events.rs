use glam::DVec3;
use minecraftoss_core::datapack::DataPack;
use minecraftoss_core::ident::Identifier;
use minecraftoss_core::tags::TagId;
use minecraftoss_core::{BlockStateId, Registries};
use serde_json::Value;
use std::collections::HashSet;

pub const STEP: &str = "minecraft:step";
pub const SWIM: &str = "minecraft:swim";
pub const SPLASH: &str = "minecraft:splash";
pub const HIT_GROUND: &str = "minecraft:hit_ground";
pub const BLOCK_PLACE: &str = "minecraft:block_place";
pub const BLOCK_DESTROY: &str = "minecraft:block_destroy";
pub const PROJECTILE_SHOOT: &str = "minecraft:projectile_shoot";
pub const PROJECTILE_LAND: &str = "minecraft:projectile_land";
pub const EXPLODE: &str = "minecraft:explode";
pub const TENDRILS_CLICKING: &str = "minecraft:sculk_sensor_tendrils_clicking";
pub const DEFAULT_RADIUS: i32 = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub id: &'static str,
    pub frequency: i32,
    pub radius: i32,
}

const fn heard(id: &'static str, frequency: i32) -> Event {
    Event {
        id,
        frequency,
        radius: DEFAULT_RADIUS,
    }
}

const fn reaching(id: &'static str, frequency: i32, radius: i32) -> Event {
    Event {
        id,
        frequency,
        radius,
    }
}

pub const EVENTS: &[Event] = &[
    heard("minecraft:block_activate", 10),
    heard("minecraft:block_attach", 10),
    heard("minecraft:block_change", 11),
    heard("minecraft:block_close", 9),
    heard("minecraft:block_deactivate", 9),
    heard(BLOCK_DESTROY, 12),
    heard("minecraft:block_detach", 9),
    heard("minecraft:block_open", 10),
    heard(BLOCK_PLACE, 13),
    heard("minecraft:bounce", 2),
    heard("minecraft:container_close", 9),
    heard("minecraft:container_open", 10),
    heard("minecraft:drink", 8),
    heard("minecraft:eat", 8),
    heard("minecraft:elytra_glide", 4),
    heard("minecraft:entity_damage", 7),
    heard("minecraft:entity_die", 15),
    heard("minecraft:entity_dismount", 5),
    heard("minecraft:entity_interact", 6),
    heard("minecraft:entity_mount", 6),
    heard("minecraft:entity_place", 14),
    heard("minecraft:entity_action", 4),
    heard("minecraft:equip", 5),
    heard(EXPLODE, 15),
    heard("minecraft:flap", 1),
    heard("minecraft:fluid_pickup", 12),
    heard("minecraft:fluid_place", 13),
    heard(HIT_GROUND, 2),
    heard("minecraft:instrument_play", 3),
    heard("minecraft:item_interact_finish", 3),
    heard("minecraft:item_interact_start", 0),
    reaching("minecraft:jukebox_play", 0, 10),
    reaching("minecraft:jukebox_stop_play", 0, 10),
    heard("minecraft:lightning_strike", 14),
    heard("minecraft:note_block_play", 10),
    heard("minecraft:prime_fuse", 10),
    heard(PROJECTILE_LAND, 2),
    heard(PROJECTILE_SHOOT, 3),
    heard(TENDRILS_CLICKING, 0),
    heard("minecraft:shear", 6),
    reaching("minecraft:shriek", 0, 32),
    heard(SPLASH, 2),
    heard(STEP, 1),
    heard(SWIM, 1),
    heard("minecraft:teleport", 14),
    heard("minecraft:unequip", 4),
    heard("minecraft:resonate_1", 1),
    heard("minecraft:resonate_2", 2),
    heard("minecraft:resonate_3", 3),
    heard("minecraft:resonate_4", 4),
    heard("minecraft:resonate_5", 5),
    heard("minecraft:resonate_6", 6),
    heard("minecraft:resonate_7", 7),
    heard("minecraft:resonate_8", 8),
    heard("minecraft:resonate_9", 9),
    heard("minecraft:resonate_10", 10),
    heard("minecraft:resonate_11", 11),
    heard("minecraft:resonate_12", 12),
    heard("minecraft:resonate_13", 13),
    heard("minecraft:resonate_14", 14),
    heard("minecraft:resonate_15", 15),
];

pub fn find(id: &str) -> Option<&'static Event> {
    EVENTS.iter().find(|event| event.id == id)
}

pub fn frequency(id: &str) -> i32 {
    find(id).map_or(0, |event| event.frequency)
}

pub fn radius(id: &str) -> i32 {
    find(id).map_or(DEFAULT_RADIUS, |event| event.radius)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Player {
        id: u64,
        sneaking: bool,
        spectator: bool,
    },
    Mob {
        id: u64,
    },
    Item {
        id: i32,
        dampens: bool,
    },
    Entity {
        id: i32,
    },
}

impl Source {
    pub fn player(&self) -> Option<u64> {
        match *self {
            Self::Player { id, .. } => Some(id),
            _ => None,
        }
    }

    fn spectator(&self) -> bool {
        matches!(
            self,
            Self::Player {
                spectator: true,
                ..
            }
        )
    }

    fn stepping_carefully(&self) -> bool {
        matches!(self, Self::Player { sneaking: true, .. })
    }

    fn dampens(&self) -> bool {
        matches!(self, Self::Item { dampens: true, .. })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Posted {
    pub event: &'static str,
    pub pos: DVec3,
    pub source: Option<Source>,
    pub source_uuid: Option<u128>,
    pub owner: Option<Source>,
    pub owner_uuid: Option<u128>,
    pub dampened: bool,
    pub tick: i64,
}

impl Posted {
    pub fn new(event: &'static str, pos: DVec3, tick: i64) -> Self {
        Self {
            event,
            pos,
            source: None,
            source_uuid: None,
            owner: None,
            owner_uuid: None,
            dampened: false,
            tick,
        }
    }

    pub fn by(self, source: Source) -> Self {
        Self {
            source: Some(source),
            ..self
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerEvent {
    pub player: u64,
    pub event: &'static str,
    pub pos: [f64; 3],
    pub affected: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Listenable {
    Vibrations,
    WardenCanListen,
    ShriekerCanListen,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EventTags {
    vibrations: HashSet<&'static str>,
    warden_can_listen: HashSet<&'static str>,
    shrieker_can_listen: HashSet<&'static str>,
    ignore_vibrations_sneaking: HashSet<&'static str>,
    dampening_items: HashSet<String>,
    dampening_blocks: Option<TagId>,
    occluding_blocks: Option<TagId>,
}

impl EventTags {
    pub fn load(registries: &Registries) -> Result<Self, String> {
        let pack = &registries.datapack;
        let events = |name: &str| -> Result<HashSet<&'static str>, String> {
            Ok(values(pack, "game_event", name)?
                .iter()
                .filter_map(|id| find(id).map(|event| event.id))
                .collect())
        };
        Ok(Self {
            vibrations: events("minecraft:vibrations")?,
            warden_can_listen: events("minecraft:warden_can_listen")?,
            shrieker_can_listen: events("minecraft:shrieker_can_listen")?,
            ignore_vibrations_sneaking: events("minecraft:ignore_vibrations_sneaking")?,
            dampening_items: values(pack, "item", "minecraft:dampens_vibrations")?
                .into_iter()
                .collect(),
            dampening_blocks: registries.block_tags.id("minecraft:dampens_vibrations"),
            occluding_blocks: registries
                .block_tags
                .id("minecraft:occludes_vibration_signals"),
        })
    }

    pub fn of(
        vibrations: &[&'static str],
        ignore_vibrations_sneaking: &[&'static str],
        dampening_items: &[&str],
    ) -> Self {
        Self {
            vibrations: vibrations.iter().copied().collect(),
            warden_can_listen: vibrations.iter().copied().collect(),
            shrieker_can_listen: [TENDRILS_CLICKING].into_iter().collect(),
            ignore_vibrations_sneaking: ignore_vibrations_sneaking.iter().copied().collect(),
            dampening_items: dampening_items
                .iter()
                .map(|item| (*item).to_owned())
                .collect(),
            dampening_blocks: None,
            occluding_blocks: None,
        }
    }

    pub fn listens(&self, listenable: Listenable, event: &str) -> bool {
        let set = match listenable {
            Listenable::Vibrations => &self.vibrations,
            Listenable::WardenCanListen => &self.warden_can_listen,
            Listenable::ShriekerCanListen => &self.shrieker_can_listen,
        };
        set.contains(event)
    }

    pub fn ignores_sneaking(&self, event: &str) -> bool {
        self.ignore_vibrations_sneaking.contains(event)
    }

    pub fn item_dampens(&self, item: &str) -> bool {
        self.dampening_items.contains(item)
    }

    pub fn block_dampens(&self, registries: &Registries, state: BlockStateId) -> bool {
        self.dampening_blocks
            .is_some_and(|tag| registries.block_in_tag(state, tag))
    }

    pub fn block_occludes(&self, registries: &Registries, state: BlockStateId) -> bool {
        self.occluding_blocks
            .is_some_and(|tag| registries.block_in_tag(state, tag))
    }
}

fn values(pack: &DataPack, registry: &str, name: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    collect(pack, registry, name, &mut seen, &mut out)?;
    Ok(out)
}

fn collect(
    pack: &DataPack,
    registry: &str,
    name: &str,
    seen: &mut HashSet<String>,
    out: &mut Vec<String>,
) -> Result<(), String> {
    if !seen.insert(name.to_owned()) {
        return Ok(());
    }
    let id = Identifier::parse(name).map_err(|e| format!("{name}: {e}"))?;
    let json = pack.read_json(&format!("tags/{registry}"), &id)?;
    for value in json
        .get("values")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let entry = match value {
            Value::String(entry) => entry.as_str(),
            Value::Object(map) => map.get("id").and_then(Value::as_str).unwrap_or_default(),
            _ => continue,
        };
        match entry.strip_prefix('#') {
            Some(tag) => collect(pack, registry, tag, seen, out)?,
            None if !out.iter().any(|known| known == entry) => out.push(entry.to_owned()),
            None => {}
        }
    }
    Ok(())
}

pub fn is_valid(tags: &EventTags, listenable: Listenable, posted: &Posted) -> bool {
    if !tags.listens(listenable, posted.event) {
        return false;
    }
    if let Some(source) = posted.source {
        if source.spectator() {
            return false;
        }
        if source.stepping_carefully() && tags.ignores_sneaking(posted.event) {
            return false;
        }
        if source.dampens() {
            return false;
        }
    }
    !posted.dampened
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frequencies_and_radii_match_vanilla() {
        let expected: &[(&str, i32)] = &[
            ("step", 1),
            ("swim", 1),
            ("flap", 1),
            ("projectile_land", 2),
            ("hit_ground", 2),
            ("splash", 2),
            ("bounce", 2),
            ("item_interact_finish", 3),
            ("projectile_shoot", 3),
            ("instrument_play", 3),
            ("entity_action", 4),
            ("elytra_glide", 4),
            ("unequip", 4),
            ("entity_dismount", 5),
            ("equip", 5),
            ("entity_interact", 6),
            ("shear", 6),
            ("entity_mount", 6),
            ("entity_damage", 7),
            ("drink", 8),
            ("eat", 8),
            ("container_close", 9),
            ("block_close", 9),
            ("block_deactivate", 9),
            ("block_detach", 9),
            ("container_open", 10),
            ("block_open", 10),
            ("block_activate", 10),
            ("block_attach", 10),
            ("prime_fuse", 10),
            ("note_block_play", 10),
            ("block_change", 11),
            ("block_destroy", 12),
            ("fluid_pickup", 12),
            ("block_place", 13),
            ("fluid_place", 13),
            ("entity_place", 14),
            ("lightning_strike", 14),
            ("teleport", 14),
            ("entity_die", 15),
            ("explode", 15),
            ("sculk_sensor_tendrils_clicking", 0),
            ("shriek", 0),
            ("item_interact_start", 0),
            ("jukebox_play", 0),
            ("jukebox_stop_play", 0),
        ];
        for (name, heard) in expected {
            let id = format!("minecraft:{name}");
            assert_eq!(frequency(&id), *heard, "{id}");
        }
        for n in 1..=15 {
            let id = format!("minecraft:resonate_{n}");
            assert_eq!(frequency(&id), n, "{id}");
            assert_eq!(radius(&id), 16, "{id}");
        }
        assert_eq!(EVENTS.len(), expected.len() + 15);
        assert_eq!(radius("minecraft:jukebox_play"), 10);
        assert_eq!(radius("minecraft:jukebox_stop_play"), 10);
        assert_eq!(radius("minecraft:shriek"), 32);
        assert!(
            EVENTS
                .iter()
                .filter(|event| !event.id.contains("jukebox") && event.id != "minecraft:shriek")
                .all(|event| event.radius == 16)
        );
        assert_eq!(frequency("minecraft:not_an_event"), 0);
    }

    #[test]
    fn the_tags_load_from_the_data_pack() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = Registries::load(&paths) else {
            return;
        };
        let tags = EventTags::load(&registries).expect("the vibration tags");
        let vibrations = Listenable::Vibrations;
        let warden = Listenable::WardenCanListen;
        let shrieker = Listenable::ShriekerCanListen;
        for heard in [STEP, BLOCK_PLACE, "minecraft:flap", "minecraft:resonate_15"] {
            assert!(tags.listens(vibrations, heard), "{heard}");
        }
        for unheard in [
            "minecraft:shriek",
            TENDRILS_CLICKING,
            "minecraft:item_interact_start",
        ] {
            assert!(!tags.listens(vibrations, unheard), "{unheard}");
        }
        assert!(!tags.listens(warden, "minecraft:flap"));
        assert!(tags.listens(warden, "minecraft:shriek"));
        assert!(tags.listens(warden, TENDRILS_CLICKING));
        assert!(tags.listens(warden, STEP));
        assert!(tags.listens(shrieker, TENDRILS_CLICKING));
        assert!(!tags.listens(shrieker, STEP));
        for careful in [
            HIT_GROUND,
            PROJECTILE_SHOOT,
            STEP,
            SWIM,
            "minecraft:item_interact_start",
            "minecraft:item_interact_finish",
        ] {
            assert!(tags.ignores_sneaking(careful), "{careful}");
        }
        assert!(!tags.ignores_sneaking(BLOCK_PLACE));
        assert!(!tags.ignores_sneaking(PROJECTILE_LAND));
        assert!(tags.item_dampens("minecraft:white_wool"));
        assert!(tags.item_dampens("minecraft:red_carpet"));
        assert!(!tags.item_dampens("minecraft:stone"));
        let state = |name: &str| registries.blocks.parse_state(name).expect(name);
        assert!(tags.block_dampens(&registries, state("minecraft:white_carpet")));
        assert!(tags.block_dampens(&registries, state("minecraft:lime_wool")));
        assert!(!tags.block_dampens(&registries, state("minecraft:stone")));
        assert!(tags.block_occludes(&registries, state("minecraft:white_wool")));
        assert!(!tags.block_occludes(&registries, state("minecraft:white_carpet")));
        assert!(!tags.block_occludes(&registries, state("minecraft:stone")));
    }

    #[test]
    fn sneaking_spectating_and_dampening_sources_are_not_valid() {
        let tags = EventTags::of(&[STEP, BLOCK_PLACE], &[STEP], &["minecraft:white_wool"]);
        let at = DVec3::new(0.5, 64.0, 0.5);
        let player = |sneaking, spectator| Source::Player {
            id: 3,
            sneaking,
            spectator,
        };
        let valid = |posted: Posted| is_valid(&tags, Listenable::Vibrations, &posted);
        assert!(valid(Posted::new(STEP, at, 0).by(player(false, false))));
        assert!(!valid(Posted::new(STEP, at, 0).by(player(true, false))));
        assert!(valid(
            Posted::new(BLOCK_PLACE, at, 0).by(player(true, false))
        ));
        assert!(!valid(
            Posted::new(BLOCK_PLACE, at, 0).by(player(false, true))
        ));
        assert!(!valid(Posted::new(STEP, at, 0).by(Source::Item {
            id: 1,
            dampens: true
        })));
        assert!(valid(Posted::new(STEP, at, 0).by(Source::Mob { id: 9 })));
        assert!(!valid(Posted {
            dampened: true,
            ..Posted::new(BLOCK_PLACE, at, 0)
        }));
        assert!(!valid(Posted::new("minecraft:shriek", at, 0)));
    }
}
