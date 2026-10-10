use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use anyhow::Result;
use minecraftoss_player::{Block, Face, Player, Pos, World};
use serde_json::Value;

use crate::bucket::{Change, ItemUse, Sound, aim, centre, offset};
use crate::pack::ResourceId;
use crate::terrain::BlockStates;

const TRANSFORMER_FOLDER: &str = "data/minecraft/block_transformer/";
const AIR: &str = "minecraft:air";
const SOUNDS: [&str; 5] = [
    "minecraft:item.shovel.flatten",
    "minecraft:item.hoe.till",
    "minecraft:item.axe.strip",
    "minecraft:item.axe.scrape",
    "minecraft:item.axe.wax_off",
];
const TOOLS: [(&str, &str); 3] = [
    ("minecraft:shovel", "minecraft:shovels"),
    ("minecraft:hoe", "minecraft:hoes"),
    ("minecraft:axe", "minecraft:axes"),
];

pub trait BlockFacts {
    fn in_tag(&self, block: Option<&Block>, tag: &str) -> bool;
    fn has_property(&self, block: &Block, name: &str) -> bool;
}

impl BlockFacts for BlockStates {
    fn in_tag(&self, block: Option<&Block>, tag: &str) -> bool {
        let registries = self.registries();
        let Some(tag) = registries.block_tags.id(tag) else {
            return false;
        };
        let id = block.map_or(AIR, |block| block.id.as_str());
        let Ok(id) = ResourceId::parse(id) else {
            return false;
        };
        let properties = block
            .map(|block| block.properties.clone())
            .unwrap_or_default();
        self.state_of(&crate::scene::Block { id, properties })
            .is_some_and(|state| registries.block_in_tag(state, tag))
    }

    fn has_property(&self, block: &Block, name: &str) -> bool {
        let Ok(id) = ResourceId::parse(&block.id) else {
            return false;
        };
        let scene = crate::scene::Block {
            id,
            properties: Default::default(),
        };
        self.state_of(&scene)
            .is_some_and(|state| self.registries().blocks.property(state, name).is_some())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Predicate {
    Tag(String, Pos),
    Blocks(Vec<String>, Pos),
    Fluids(Vec<String>, Pos),
    Unobstructed,
    All(Vec<Predicate>),
    Any(Vec<Predicate>),
    Not(Box<Predicate>),
    True,
}

fn ids(json: &Value) -> Option<Vec<String>> {
    match json {
        Value::String(id) => Some(vec![id.clone()]),
        Value::Array(ids) => ids
            .iter()
            .map(|id| id.as_str().map(str::to_owned))
            .collect(),
        _ => None,
    }
}

fn fluid_of(block: Option<&Block>) -> Option<&'static str> {
    let block = block?;
    let source = block.property("level").is_none_or(|level| level == "0");
    match block.id.as_str() {
        "minecraft:water" if source => Some("minecraft:water"),
        "minecraft:water" => Some("minecraft:flowing_water"),
        "minecraft:lava" if source => Some("minecraft:lava"),
        "minecraft:lava" => Some("minecraft:flowing_lava"),
        "minecraft:kelp"
        | "minecraft:kelp_plant"
        | "minecraft:seagrass"
        | "minecraft:tall_seagrass"
        | "minecraft:bubble_column" => Some("minecraft:water"),
        _ if block.property("waterlogged") == Some("true") => Some("minecraft:water"),
        _ => None,
    }
}

fn fluid_matches(wanted: &str, fluid: &str) -> bool {
    match wanted {
        "#minecraft:water" => matches!(fluid, "minecraft:water" | "minecraft:flowing_water"),
        "#minecraft:lava" => matches!(fluid, "minecraft:lava" | "minecraft:flowing_lava"),
        _ => wanted == fluid,
    }
}

impl Predicate {
    pub(crate) fn parse(json: &Value) -> Option<Self> {
        let offset = || -> Option<Pos> {
            match json.get("offset") {
                None => Some((0, 0, 0)),
                Some(offset) => {
                    let at = |i: usize| offset.get(i)?.as_i64().and_then(|v| i32::try_from(v).ok());
                    Some((at(0)?, at(1)?, at(2)?))
                }
            }
        };
        let list = |key: &str| -> Option<Vec<Self>> {
            json[key].as_array()?.iter().map(Self::parse).collect()
        };
        match json["type"].as_str()? {
            "minecraft:matching_block_tag" => {
                Some(Self::Tag(json["tag"].as_str()?.to_owned(), offset()?))
            }
            "minecraft:matching_blocks" => Some(Self::Blocks(ids(&json["blocks"])?, offset()?)),
            "minecraft:matching_fluids" => Some(Self::Fluids(ids(&json["fluids"])?, offset()?)),
            "minecraft:unobstructed" if json.get("offset").is_none() => Some(Self::Unobstructed),
            "minecraft:all_of" => Some(Self::All(list("predicates")?)),
            "minecraft:any_of" => Some(Self::Any(list("predicates")?)),
            "minecraft:not" => Some(Self::Not(Box::new(Self::parse(&json["predicate"])?))),
            "minecraft:true" => Some(Self::True),
            _ => None,
        }
    }

    pub(crate) fn test(
        &self,
        world: &impl World,
        facts: &impl BlockFacts,
        pos: Pos,
        unobstructed: &dyn Fn(Pos) -> bool,
    ) -> bool {
        match self {
            Self::Tag(tag, by) => facts.in_tag(world.block(offset(pos, *by)).as_ref(), tag),
            Self::Blocks(ids, by) => {
                let block = world.block(offset(pos, *by));
                let id = block.as_ref().map_or(AIR, |block| block.id.as_str());
                ids.iter().any(|wanted| match wanted.strip_prefix('#') {
                    Some(tag) => facts.in_tag(block.as_ref(), tag),
                    None => wanted == id,
                })
            }
            Self::Fluids(wanted, by) => fluid_of(world.block(offset(pos, *by)).as_ref())
                .is_some_and(|fluid| wanted.iter().any(|wanted| fluid_matches(wanted, fluid))),
            Self::Unobstructed => unobstructed(pos),
            Self::All(all) => all.iter().all(|p| p.test(world, facts, pos, unobstructed)),
            Self::Any(any) => any.iter().any(|p| p.test(world, facts, pos, unobstructed)),
            Self::Not(inner) => !inner.test(world, facts, pos, unobstructed),
            Self::True => true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Provider {
    State(Block),
    Rules(Vec<(Predicate, Provider)>, Option<Box<Provider>>),
    CopyProperties(Box<Provider>),
}

impl Provider {
    fn parse(json: &Value) -> Option<Self> {
        match json.get("type").and_then(Value::as_str) {
            None => Self::state(json),
            Some("minecraft:simple_state_provider") => Self::state(&json["state"]),
            Some("minecraft:rule_based") => {
                let rules = json["rules"]
                    .as_array()?
                    .iter()
                    .map(|rule| {
                        Some((
                            Predicate::parse(&rule["if_true"])?,
                            Self::parse(&rule["then"])?,
                        ))
                    })
                    .collect::<Option<_>>()?;
                let fallback = match json.get("fallback") {
                    None => None,
                    Some(fallback) => Some(Box::new(Self::parse(fallback)?)),
                };
                Some(Self::Rules(rules, fallback))
            }
            Some("minecraft:copy_properties") => Some(Self::CopyProperties(Box::new(Self::parse(
                &json["source"],
            )?))),
            Some(_) => None,
        }
    }

    fn state(json: &Value) -> Option<Self> {
        let id = json.get("id").or_else(|| json.get("Name"))?.as_str()?;
        let mut block = Block::new(id);
        if let Some(properties) = json.get("properties").or_else(|| json.get("Properties")) {
            for (name, value) in properties.as_object()? {
                block = block.with(name, value.as_str()?);
            }
        }
        Some(Self::State(block))
    }

    fn state_at(&self, world: &impl World, facts: &impl BlockFacts, pos: Pos) -> Option<Block> {
        match self {
            Self::State(block) => Some(block.clone()),
            Self::Rules(rules, fallback) => rules
                .iter()
                .find(|(predicate, _)| predicate.test(world, facts, pos, &|_| true))
                .map_or_else(
                    || fallback.as_ref()?.state_at(world, facts, pos),
                    |(_, then)| then.state_at(world, facts, pos),
                ),
            Self::CopyProperties(source) => {
                let mut state = source.state_at(world, facts, pos)?;
                if let Some(clicked) = world.block(pos) {
                    for (name, value) in &clicked.properties {
                        if facts.has_property(&state, name) {
                            state = state.with(name, value);
                        }
                    }
                }
                Some(state)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Transform {
    provider: Provider,
    disallowed_faces: Vec<Face>,
    damage: u32,
    sound: Option<&'static str>,
}

impl Transform {
    fn parse(json: &Value) -> Option<Self> {
        let unsupported = json.get("loot").is_some()
            || json.get("transform_type").is_some()
            || json.get("consume_on_use").and_then(Value::as_bool) == Some(true)
            || json.get("update_from_neighbors").and_then(Value::as_bool) == Some(false);
        if unsupported {
            return None;
        }
        let disallowed_faces = match json.get("disallowed_faces") {
            None => Vec::new(),
            Some(faces) => faces
                .as_array()?
                .iter()
                .map(|face| face_named(face.as_str()?))
                .collect::<Option<_>>()?,
        };
        let sound = json["sound"]
            .as_str()
            .and_then(|sound| SOUNDS.iter().copied().find(|known| *known == sound));
        Some(Self {
            provider: Provider::parse(&json["block_state_provider"])?,
            disallowed_faces,
            damage: json["item_damage_per_use"].as_u64().unwrap_or(0) as u32,
            sound,
        })
    }
}

fn face_named(name: &str) -> Option<Face> {
    Some(match name {
        "down" => Face::Down,
        "up" => Face::Up,
        "north" => Face::North,
        "south" => Face::South,
        "west" => Face::West,
        "east" => Face::East,
        _ => return None,
    })
}

#[derive(Clone, Debug, Default)]
pub struct TransformBook {
    tools: HashMap<String, Vec<Transform>>,
}

impl TransformBook {
    pub fn from_jar(jar: &Path) -> Result<Self> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
        let mut files = Vec::new();
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index)?;
            let name = entry.name().to_owned();
            let Some(path) = name
                .strip_prefix(TRANSFORMER_FOLDER)
                .and_then(|rest| rest.strip_suffix(".json"))
            else {
                continue;
            };
            let id = format!("minecraft:{path}");
            let mut text = String::new();
            entry.read_to_string(&mut text)?;
            files.push((id, serde_json::from_str::<Value>(&text)?));
        }
        Ok(Self::from_json(files))
    }

    pub fn from_json(files: Vec<(String, Value)>) -> Self {
        let tools = files
            .into_iter()
            .map(|(id, json)| {
                let transforms = json
                    .as_array()
                    .map(|entries| entries.iter().filter_map(Transform::parse).collect())
                    .unwrap_or_default();
                (id, transforms)
            })
            .collect();
        Self { tools }
    }

    pub fn transform_use(
        &self,
        tool_tag: impl Fn(&str) -> bool,
        player: &Player,
        world: &impl World,
        facts: &impl BlockFacts,
        shield_ready: bool,
    ) -> Option<ItemUse> {
        if shield_ready {
            return None;
        }
        let transforms = TOOLS
            .iter()
            .find(|(_, tag)| tool_tag(tag))
            .and_then(|(id, _)| self.tools.get(*id))?;
        let hit = aim(player, world, false)?;
        transforms
            .iter()
            .filter(|transform| !transform.disallowed_faces.contains(&hit.face))
            .find_map(|transform| {
                let state = transform.provider.state_at(world, facts, hit.pos)?;
                Some(ItemUse {
                    change: Change::Set(hit.pos, state),
                    result: None,
                    wear: transform.damage,
                    consumed: false,
                    sounds: transform
                        .sound
                        .map(|event| Sound {
                            event,
                            at: Some(centre(hit.pos)),
                            volume: 1.0,
                            pitch: 1.0,
                        })
                        .into_iter()
                        .collect(),
                })
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::DVec3;
    use serde_json::json;

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

    struct Facts;

    impl BlockFacts for Facts {
        fn in_tag(&self, block: Option<&Block>, tag: &str) -> bool {
            let id = block.map_or(AIR, |block| block.id.as_str());
            match tag {
                "minecraft:air" => id == AIR,
                "minecraft:turns_into_dirt_path" => {
                    matches!(id, "minecraft:grass_block" | "minecraft:dirt")
                }
                "minecraft:turns_into_farmland" => {
                    matches!(id, "minecraft:grass_block" | "minecraft:dirt")
                }
                _ => false,
            }
        }

        fn has_property(&self, block: &Block, name: &str) -> bool {
            block.id.ends_with("_log") && name == "axis"
        }
    }

    fn air_above(tag: &str) -> Value {
        json!({
            "type": "minecraft:all_of",
            "predicates": [
                {"type": "minecraft:matching_block_tag", "tag": tag},
                {"type": "minecraft:matching_block_tag", "offset": [0, 1, 0], "tag": "minecraft:air"}
            ]
        })
    }

    fn book() -> TransformBook {
        TransformBook::from_json(vec![
            (
                "minecraft:shovel".to_owned(),
                json!([{
                    "block_state_provider": {"type": "minecraft:rule_based", "rules": [
                        {"if_true": air_above("minecraft:turns_into_dirt_path"), "then": {"id": "minecraft:dirt_path"}}
                    ]},
                    "disallowed_faces": ["down"],
                    "item_damage_per_use": 1,
                    "sound": "minecraft:item.shovel.flatten"
                }]),
            ),
            (
                "minecraft:hoe".to_owned(),
                json!([
                    {
                        "block_state_provider": {"type": "minecraft:rule_based", "rules": [
                            {"if_true": air_above("minecraft:turns_into_farmland"), "then": {"id": "minecraft:farmland", "properties": {"moisture": "0"}}},
                            {"if_true": {"type": "minecraft:all_of", "predicates": [
                                {"type": "minecraft:matching_blocks", "blocks": "minecraft:coarse_dirt"},
                                {"type": "minecraft:matching_block_tag", "offset": [0, 1, 0], "tag": "minecraft:air"}
                            ]}, "then": {"id": "minecraft:dirt"}}
                        ]},
                        "disallowed_faces": ["down"],
                        "item_damage_per_use": 1,
                        "sound": "minecraft:item.hoe.till"
                    },
                    {
                        "block_state_provider": {"type": "minecraft:rule_based", "rules": [
                            {"if_true": {"type": "minecraft:matching_blocks", "blocks": "minecraft:rooted_dirt"}, "then": {"id": "minecraft:dirt"}}
                        ]},
                        "drop_strategy": "clicked_face",
                        "loot": "minecraft:till/rooted_dirt",
                        "sound": "minecraft:item.hoe.till"
                    }
                ]),
            ),
            (
                "minecraft:axe".to_owned(),
                json!([
                    {
                        "block_state_provider": {"type": "minecraft:rule_based", "rules": [
                            {"if_true": {"type": "minecraft:matching_blocks", "blocks": "minecraft:oak_log"}, "then": {"type": "minecraft:copy_properties", "source": {"id": "minecraft:stripped_oak_log", "properties": {"axis": "y"}}}}
                        ]},
                        "item_damage_per_use": 1,
                        "sound": "minecraft:item.axe.strip"
                    },
                    {
                        "block_state_provider": {"type": "minecraft:rule_based", "rules": [
                            {"if_true": {"type": "minecraft:matching_blocks", "blocks": "minecraft:exposed_copper_door"}, "then": {"id": "minecraft:copper_door"}}
                        ]},
                        "item_damage_per_use": 1,
                        "update_from_neighbors": false
                    }
                ]),
            ),
        ])
    }

    fn aiming_at(blocks: &[(Pos, Block)], pitch: f64) -> (Player, Blocks) {
        let mut world = Blocks::default();
        for (pos, block) in blocks {
            world.set_block(*pos, Some(block.clone()));
        }
        let mut player = Player::new(DVec3::new(0.5, 1.0, -1.5));
        player.yaw = 0.0;
        player.pitch = pitch;
        (player, world)
    }

    fn tool(tag: &'static str) -> impl Fn(&str) -> bool {
        move |wanted| wanted == tag
    }

    #[test]
    fn a_hoe_tills_grass_and_dirt_under_air_and_coarse_dirt_into_dirt() {
        let book = book();
        for (ground, tilled) in [
            ("minecraft:grass_block", "minecraft:farmland"),
            ("minecraft:dirt", "minecraft:farmland"),
            ("minecraft:coarse_dirt", "minecraft:dirt"),
        ] {
            let (player, world) = aiming_at(&[((0, 0, 0), Block::new(ground))], 45.0);
            let used = book
                .transform_use(tool("minecraft:hoes"), &player, &world, &Facts, false)
                .unwrap();
            let Change::Set(pos, block) = &used.change else {
                panic!("{ground} was not set");
            };
            assert_eq!((*pos, block.id.as_str()), ((0, 0, 0), tilled));
            assert_eq!(
                (used.wear, used.consumed, used.result.is_none()),
                (1, false, true)
            );
            assert_eq!(
                used.sounds,
                vec![Sound::at_block("minecraft:item.hoe.till", (0, 0, 0))]
            );
        }
        let (player, world) = aiming_at(&[((0, 0, 0), Block::new("minecraft:dirt"))], 45.0);
        let used = book
            .transform_use(tool("minecraft:hoes"), &player, &world, &Facts, false)
            .unwrap();
        assert_eq!(
            used.change,
            Change::Set(
                (0, 0, 0),
                Block::new("minecraft:farmland").with("moisture", "0")
            )
        );
        let covered = [
            ((0, 0, 0), Block::new("minecraft:dirt")),
            ((0, 1, 0), Block::new("minecraft:short_grass")),
        ];
        let (player, world) = aiming_at(&covered, 45.0);
        assert!(
            book.transform_use(tool("minecraft:hoes"), &player, &world, &Facts, false)
                .is_none()
        );
        let (player, world) = aiming_at(&[((0, 0, 0), Block::new("minecraft:rooted_dirt"))], 45.0);
        assert!(
            book.transform_use(tool("minecraft:hoes"), &player, &world, &Facts, false)
                .is_none(),
            "rooted dirt drops loot, which is left out"
        );
    }

    #[test]
    fn a_shovel_flattens_a_path_but_not_from_below_and_not_with_a_shield_up() {
        let book = book();
        let grass = [((0, 0, 0), Block::new("minecraft:grass_block"))];
        let (player, world) = aiming_at(&grass, 45.0);
        let used = book
            .transform_use(tool("minecraft:shovels"), &player, &world, &Facts, false)
            .unwrap();
        assert_eq!(
            used.change,
            Change::Set((0, 0, 0), Block::new("minecraft:dirt_path"))
        );
        assert_eq!(used.sounds[0].event, "minecraft:item.shovel.flatten");
        assert!(
            book.transform_use(tool("minecraft:shovels"), &player, &world, &Facts, true)
                .is_none()
        );
        let ceiling = [((0, 4, 0), Block::new("minecraft:grass_block"))];
        let (mut player, world) = aiming_at(&ceiling, -45.0);
        player.pos = DVec3::new(0.5, 1.0, -1.0);
        assert_eq!(
            player.target(&world, 5.0).map(|hit| hit.face),
            Some(Face::Down)
        );
        assert!(
            book.transform_use(tool("minecraft:shovels"), &player, &world, &Facts, false)
                .is_none()
        );
        assert!(
            book.transform_use(tool("minecraft:swords"), &player, &world, &Facts, false)
                .is_none()
        );
    }

    #[test]
    fn an_axe_strips_a_log_keeping_its_axis_and_leaves_doors_alone() {
        let book = book();
        let log = Block::new("minecraft:oak_log").with("axis", "x");
        let (player, world) = aiming_at(&[((0, 0, 0), log)], 45.0);
        let used = book
            .transform_use(tool("minecraft:axes"), &player, &world, &Facts, false)
            .unwrap();
        assert_eq!(
            used.change,
            Change::Set(
                (0, 0, 0),
                Block::new("minecraft:stripped_oak_log").with("axis", "x")
            )
        );
        let door = [((0, 0, 0), Block::new("minecraft:exposed_copper_door"))];
        let (player, world) = aiming_at(&door, 45.0);
        assert!(
            book.transform_use(tool("minecraft:axes"), &player, &world, &Facts, false)
                .is_none()
        );
    }

    #[test]
    fn the_block_catalog_answers_tags_and_properties() {
        let Ok(paths) = minecraftoss_core::registries::DataPaths::discover() else {
            return;
        };
        let Ok(registries) = minecraftoss_core::Registries::load(&paths) else {
            return;
        };
        let states = BlockStates::new(std::sync::Arc::new(registries), 0, -64, 384).unwrap();
        let grass = Block::new("minecraft:grass_block");
        assert!(states.in_tag(Some(&grass), "minecraft:turns_into_dirt_path"));
        assert!(states.in_tag(Some(&grass), "minecraft:turns_into_farmland"));
        assert!(!states.in_tag(
            Some(&Block::new("minecraft:stone")),
            "minecraft:turns_into_farmland"
        ));
        assert!(states.in_tag(None, "minecraft:air"));
        assert!(states.in_tag(Some(&Block::new("minecraft:cave_air")), "minecraft:air"));
        assert!(!states.in_tag(Some(&grass), "minecraft:air"));
        assert!(states.has_property(&Block::new("minecraft:stripped_oak_log"), "axis"));
        assert!(!states.has_property(&Block::new("minecraft:copper_block"), "axis"));
    }

    #[test]
    fn the_vanilla_transformers_read_from_the_jar() {
        let Ok(root) = std::env::var("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let book = TransformBook::from_jar(&jar).unwrap();
        assert_eq!(book.tools["minecraft:shovel"].len(), 1);
        assert_eq!(book.tools["minecraft:hoe"].len(), 1);
        assert_eq!(book.tools["minecraft:axe"].len(), 3);
        let till = &book.tools["minecraft:hoe"][0];
        assert_eq!(
            (till.disallowed_faces.as_slice(), till.damage, till.sound),
            (&[Face::Down][..], 1, Some("minecraft:item.hoe.till"))
        );
    }
}
