use std::any::Any;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use minecraft_commands::{
    Answer, BlockEdit, Clientbound, Clock, CommandData, CommandHints, CommandRegistries,
    CommandServer, CommandWorld, Enchanted, EnchantmentInfo, Experience, ItemArg, ItemFilter,
    Loaded, LocateKind, LocateRequest, Located, PlayerId, Pose, Serverbound, Text, Weather,
};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::locate::Search;
use minecraft_terrain::server::{ServerSim, THE_END_CLOCK};
use minecraft_terrain::terrain::TerrainStream;
use minecraft_terrain::game_mode::GameMode;
use minecraft_terrain::game_rules::{GameRules, Rule, Value};
use minecraftoss_player::Difficulty;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{Armor, EffectKind};

use crate::command_blocks::SimBlocks;
use crate::entities::Entities;
use crate::level::Level;
use crate::player::Player;
use crate::server::Globals;

pub const HOST: PlayerId = PlayerId(0);
pub const LEVEL_UP_SOUND: &str = "minecraft:entity.player.levelup";
pub const LOCATE_TIME_CAP: std::time::Duration = std::time::Duration::from_secs(20);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Presence {
    pub pose: Option<Pose>,
    pub alive: bool,
}

pub struct Host<'a> {
    pub seed: i64,
    pub rules: &'a mut GameRules,
    pub difficulty: &'a mut Difficulty,
    pub day: &'a mut DayCycle,
    pub end_clock: &'a mut DayCycle,
    pub save: Option<&'a mut crate::save::WorldSave>,
    pub data: &'a crate::data::GameData,
    pub registries: Option<&'a minecraftoss_core::Registries>,
    pub levels: &'a mut BTreeMap<Dimension, Level>,
    pub waiting_locates: &'a mut Vec<(Dimension, PlayerId, LocateRequest)>,
    pub players: &'a mut BTreeMap<PlayerUuid, Player>,
    pub presence: Vec<(PlayerUuid, Presence)>,
    pub env_file: Option<PathBuf>,
    pub rules_changed: bool,
    pub mode_changed: Vec<PlayerId>,
    pub respawn_changed: Vec<PlayerId>,
    pub teleports: Vec<(PlayerId, [f64; 3])>,
}

pub fn env_file() -> Option<PathBuf> {
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| Some(exe.parent()?.join(".env")));
    if let Some(candidate) = beside_exe.filter(|candidate| candidate.is_file()) {
        return Some(candidate);
    }
    let mut dir = std::env::current_dir().ok()?;
    loop {
        let candidate = dir.join(".env");
        if candidate.is_file() {
            return Some(candidate);
        }
        if !dir.pop() {
            return None;
        }
    }
}

impl Host<'_> {
    fn player(&self, id: PlayerId) -> Option<&Player> {
        self.players.values().find(|player| player.id == id)
    }

    fn player_mut(&mut self, id: PlayerId) -> Option<&mut Player> {
        self.players.values_mut().find(|player| player.id == id)
    }

    fn presence_mut(&mut self, id: PlayerId) -> Option<&mut Presence> {
        let uuid = self.player(id)?.uuid.clone();
        self.presence
            .iter_mut()
            .find(|(known, _)| *known == uuid)
            .map(|(_, presence)| presence)
    }

    fn presence(&self, id: PlayerId) -> Presence {
        let Some(player) = self.player(id) else {
            return Presence::default();
        };
        self.presence
            .iter()
            .find(|(known, _)| *known == player.uuid)
            .map_or_else(Presence::default, |(_, presence)| *presence)
    }

    fn player_in_level(&mut self, id: PlayerId) -> Option<(&mut Player, Option<&mut Entities>)> {
        let player = self.players.values_mut().find(|player| player.id == id)?;
        let entities = self
            .levels
            .get_mut(&player.dimension)
            .and_then(|level| level.entities.as_mut());
        Some((player, entities))
    }

    fn entities_of(&mut self, id: PlayerId) -> Option<&mut Entities> {
        let dimension = self.player(id).map_or(Dimension::Overworld, |player| player.dimension);
        self.levels.get_mut(&dimension)?.entities.as_mut()
    }
}

pub fn registries(packs: &minecraft_terrain::pack::PackStack) -> CommandRegistries {
    let mut items: Vec<String> = packs
        .list("minecraft", "items/")
        .unwrap_or_else(|_| Vec::new())
        .iter()
        .filter_map(|path| {
            path.strip_prefix("assets/minecraft/items/")?
                .strip_suffix(".json")
        })
        .filter(|name| !name.contains('/'))
        .map(|name| format!("minecraft:{name}"))
        .collect();
    items.sort();
    let mut effects: Vec<String> = minecraft_terrain::item_icons::language(packs)
        .unwrap_or_else(|_| HashMap::new())
        .keys()
        .filter_map(|key| key.strip_prefix("effect.minecraft."))
        .filter(|name| !name.contains('.'))
        .map(|name| format!("minecraft:{name}"))
        .collect();
    effects.sort();
    CommandRegistries {
        items,
        entities: minecraft_terrain::server_mobs::SIMULATED_KINDS
            .iter()
            .map(|kind| (*kind).to_owned())
            .collect(),
        effects,
        enchantments: Vec::new(),
    }
}

pub fn effect_kind(effect: &str) -> Option<EffectKind> {
    Some(match effect.strip_prefix("minecraft:")? {
        "hunger" => EffectKind::Hunger,
        "poison" => EffectKind::Poison,
        "regeneration" => EffectKind::Regeneration,
        "absorption" => EffectKind::Absorption,
        "resistance" => EffectKind::Resistance,
        "fire_resistance" => EffectKind::FireResistance,
        "nausea" => EffectKind::Nausea,
        "slowness" => EffectKind::Slowness,
        "weakness" => EffectKind::Weakness,
        "wither" => EffectKind::Wither,
        _ => return None,
    })
}

pub fn give_stacks(
    inventory: &mut Inventory,
    selected: usize,
    stack: &ItemStack,
    count: u32,
) -> (Vec<ItemStack>, u32) {
    let max = u32::from(stack.max.max(1));
    let mut remaining = count;
    let mut dropped = Vec::new();
    let mut added = 0;
    while remaining > 0 {
        let size = remaining.min(max);
        remaining -= size;
        match inventory.add_item(
            ItemStack {
                count: size as u8,
                ..stack.clone()
            },
            selected,
        ) {
            None => added += 1,
            Some(rest) => dropped.push(rest),
        }
    }
    (dropped, added)
}

fn nbt_error(error: &str) -> Text {
    let key = if error.contains("expected a compound") {
        "argument.nbt.expected.compound"
    } else if error.contains("trailing data") {
        "argument.nbt.trailing"
    } else if error.contains("empty key") {
        "argument.nbt.expected.key"
    } else if error.contains("expected a value") {
        "argument.nbt.expected.value"
    } else {
        return Text::literal(error);
    };
    Text::translate(key, Vec::new())
}

pub fn summon_answers(
    server: &CommandServer,
    rules: &GameRules,
    answers: Vec<Result<String, String>>,
) -> Vec<Clientbound> {
    let level = server.level(HOST);
    answers
        .into_iter()
        .flat_map(|answer| match answer {
            Ok(kind) => server.feedback(
                HOST,
                Text::translate("commands.summon.success", vec![Text::entity(kind)]),
                true,
                rules,
            ),
            Err(error) => {
                let key = if error.contains("peaceful") {
                    "commands.summon.failed.peaceful"
                } else if error.contains("Invalid position") {
                    "commands.summon.invalidPosition"
                } else {
                    "commands.summon.failed"
                };
                vec![minecraft_commands::error_line(
                    HOST,
                    Text::translate(key, Vec::new()),
                )]
            }
        })
        .filter(|out| out.to.reaches(HOST, level))
        .map(|out| out.message)
        .collect()
}

impl CommandWorld for Host<'_> {
    fn seed(&self) -> i64 {
        self.seed
    }

    fn game_rules(&self) -> &GameRules {
        self.rules
    }

    fn set_game_rule(&mut self, rule: &'static Rule, value: Value) {
        if self.rules.set(rule.id, &value.to_string()).is_ok() {
            self.rules_changed = true;
        }
    }

    fn difficulty(&self) -> Difficulty {
        *self.difficulty
    }

    fn set_difficulty(&mut self, difficulty: Difficulty) {
        *self.difficulty = difficulty;
    }

    fn clock(&self, clock: &str) -> Clock {
        let total = self.day.ticks.floor() as i64;
        let game_time = self
            .save
            .as_ref()
            .map_or(total, |save| save.game_time(Some(self.day.ticks)));
        let cycle: &DayCycle = if clock == THE_END_CLOCK {
            self.end_clock
        } else {
            self.day
        };
        Clock {
            total: cycle.ticks.floor() as i64,
            game_time,
            paused: cycle.paused,
        }
    }

    fn set_time(&mut self, clock: &str, total: i64) {
        if clock == THE_END_CLOCK {
            self.end_clock.set(total as f64);
            return;
        }
        let ticks = total as f64;
        if let Some(save) = self.save.as_mut() {
            save.shift_day(ticks - self.day.ticks);
        }
        self.day.set(ticks);
    }

    fn set_time_paused(&mut self, clock: &str, paused: bool) {
        if clock == THE_END_CLOCK {
            self.end_clock.paused = paused;
        } else {
            self.day.paused = paused;
        }
    }

    fn default_clock(&self, dimension: &str) -> Option<String> {
        crate::switch::dimension_named(dimension)
            .and_then(crate::server::default_clock)
            .map(str::to_owned)
    }

    fn set_weather(&mut self, weather: Weather, duration: i32) {
        let kind = match weather {
            Weather::Clear => "clear",
            Weather::Rain => "rain",
            Weather::Thunder => "thunder",
        };
        if let Some(entities) = self
            .levels
            .get_mut(&Dimension::Overworld)
            .and_then(|level| level.entities.as_mut())
        {
            entities.set_weather(kind, duration);
        } else if let Some(entities) = self
            .levels
            .values_mut()
            .find_map(|level| level.entities.as_mut())
        {
            entities.set_weather(kind, duration);
        }
    }

    fn kill(&mut self, player: PlayerId) -> bool {
        let Some(player) = self.player_mut(player) else {
            return false;
        };
        player.survival.kill(crate::death::DamageCause::Kill);
        true
    }

    fn game_mode(&self, player: PlayerId) -> Option<GameMode> {
        self.player(player).map(|player| player.mode.mode)
    }

    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) -> bool {
        let changed = self
            .player_mut(player)
            .is_some_and(|known| known.mode.set(mode));
        if changed {
            self.mode_changed.push(player);
        }
        changed
    }

    fn pose(&self, player: PlayerId) -> Option<Pose> {
        self.presence(player).pose
    }

    fn max_stack(&self, item: &str) -> u32 {
        u32::from(self.data.recipes.max_stack(item))
    }

    fn give(&mut self, player: PlayerId, item: &ItemArg, count: u32) -> bool {
        let feet = self.presence(player).pose.map(|pose| pose.feet);
        let stack = minecraft_terrain::crafting::kit_potion_stack(
            &self.data.recipes,
            &item.id,
            1,
            &item.enchantments,
            item.potion.as_deref(),
        );
        let Some((player, entities)) = self.player_in_level(player) else {
            return false;
        };
        let (dropped, added) =
            give_stacks(&mut player.inventory, player.selected, &stack, count);
        if let (Some(feet), Some(entities)) = (feet, entities) {
            for stack in &dropped {
                entities.drop_given(stack, feet);
            }
            for _ in 0..added {
                let pitch = ((entities.random_float() - entities.random_float()) * 0.7 + 1.0) * 2.0;
                entities.sounds.push((
                    "minecraft:entity.item.pickup".to_owned(),
                    glam::DVec3::from_array(feet),
                    0.2,
                    pitch,
                ));
            }
        }
        true
    }

    fn teleport(&mut self, player: PlayerId, position: [f64; 3]) -> bool {
        let Some(presence) = self.presence_mut(player).filter(|presence| presence.alive) else {
            return false;
        };
        if let Some(pose) = presence.pose.as_mut() {
            pose.feet = position;
        }
        self.teleports.push((player, position));
        true
    }

    fn effect_give(
        &mut self,
        player: PlayerId,
        effect: &str,
        ticks: Option<u32>,
        amplifier: u8,
    ) -> bool {
        let Some(survival) = self
            .player_mut(player)
            .map(|player| &mut player.survival)
            .filter(|survival| survival.status.health > 0.0)
        else {
            return false;
        };
        let times = ticks.unwrap_or(1).clamp(1, 40);
        match effect {
            "minecraft:instant_health" => {
                let heal = 4_i32.wrapping_shl(u32::from(amplifier)).max(0) as f32;
                survival.status.heal(heal * times as f32);
                return true;
            }
            "minecraft:instant_damage" => {
                let damage = 6_i32.wrapping_shl(u32::from(amplifier)).max(0) as f32;
                survival.hurt(
                    damage,
                    Armor::default(),
                    None,
                    crate::death::DamageCause::Magic,
                );
                return true;
            }
            "minecraft:saturation" => {
                let food = u32::from(amplifier) + 1;
                for _ in 0..times {
                    survival
                        .status
                        .food
                        .eat(food.min(20) as u8, 2.0 * food as f32);
                }
                return true;
            }
            _ => {}
        }
        let Some(kind) = effect_kind(effect) else {
            return false;
        };
        let status = &mut survival.status;
        let before = status.effect(kind);
        status.add_effect(kind, ticks.unwrap_or(u32::MAX), amplifier);
        status.effect(kind) != before
    }

    fn effect_clear(&mut self, player: PlayerId, effect: Option<&str>) -> bool {
        let Some(survival) = self.player_mut(player).map(|player| &mut player.survival) else {
            return false;
        };
        let status = &mut survival.status;
        match effect {
            Some(effect) => match effect_kind(effect).filter(|kind| status.effect(*kind).is_some())
            {
                Some(kind) => {
                    status.remove_effect(kind);
                    true
                }
                None => false,
            },
            None => {
                let kinds = [
                    EffectKind::Hunger,
                    EffectKind::Poison,
                    EffectKind::Regeneration,
                    EffectKind::Absorption,
                    EffectKind::Resistance,
                    EffectKind::FireResistance,
                    EffectKind::Nausea,
                    EffectKind::Slowness,
                    EffectKind::Weakness,
                    EffectKind::Wither,
                ];
                let any = kinds.iter().any(|kind| status.effect(*kind).is_some());
                survival.clear_effects();
                any
            }
        }
    }

    fn summon(
        &mut self,
        source: PlayerId,
        kind: &str,
        position: [f64; 3],
        nbt: Option<&str>,
    ) -> Result<(), Text> {
        let tag = nbt
            .map(minecraftoss_core::snbt::parse_compound)
            .transpose()
            .map_err(|error| nbt_error(&error))?;
        let Some(entities) = self.entities_of(source) else {
            return Err(Text::translate("commands.summon.failed", Vec::new()));
        };
        entities.summon(kind.to_owned(), position, tag);
        Ok(())
    }

    fn clear(&mut self, player: PlayerId, filter: &ItemFilter, max: i32) -> Option<i32> {
        let player = self.player_mut(player)?;
        Some(minecraft_commands::clear_inventory(
            &mut player.inventory,
            filter,
            max,
        ))
    }

    fn experience(&self, player: PlayerId) -> Option<Experience> {
        let status = &self.player(player)?.survival.status;
        Some(Experience {
            level: i32::try_from(status.experience_level).unwrap_or(i32::MAX),
            progress: status.experience_progress,
            total: i32::try_from(status.total_experience).unwrap_or(i32::MAX),
        })
    }

    fn set_experience(&mut self, player: PlayerId, experience: Experience) -> bool {
        let Some(player) = self.player_mut(player) else {
            return false;
        };
        let status = &mut player.survival.status;
        status.experience_level = experience.level.max(0) as u32;
        status.experience_progress = experience.progress;
        status.total_experience = experience.total.max(0) as u32;
        true
    }

    fn enchantment(&self, id: &str) -> Option<EnchantmentInfo> {
        let book = self.data.enchantments.as_ref()?;
        let index = book.index(id)?;
        Some(EnchantmentInfo {
            max_level: book.list[index].max_level,
            curse: book.tag_contains("minecraft:curse", id),
        })
    }

    fn enchant(&mut self, player: PlayerId, enchantment: &str, level: i32) -> Option<Enchanted> {
        let book = self.data.enchantments.clone()?;
        let wanted = book.index(enchantment)?;
        let player = self.player_mut(player)?;
        let selected = player.selected;
        let Some(stack) = player.inventory.slots.get_mut(selected).and_then(Option::as_mut) else {
            return Some(Enchanted::Itemless);
        };
        let compatible = book.list[wanted].can_enchant(&stack.id)
            && held_enchantments(stack)
                .iter()
                .all(|id| book.index(id).is_none_or(|known| book.compatible(known, wanted)));
        if !compatible {
            return Some(Enchanted::Incompatible(Text::hover_name(stack.id.clone())));
        }
        let mut components = match stack.components.take() {
            Some(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
        minecraftoss_entities::enchant::add_enchantment(
            &stack.id,
            &mut components,
            enchantment,
            level,
        );
        stack.components = Some(serde_json::Value::Object(components));
        Some(Enchanted::Applied)
    }

    fn dimension(&self, player: PlayerId) -> String {
        self.player(player)
            .map_or(Dimension::Overworld, |player| player.dimension)
            .dimension_type()
            .to_owned()
    }

    fn dimension_scale(&self, dimension: &str) -> Option<f64> {
        let dimension = crate::switch::dimension_named(dimension)?;
        let datapack = &self.registries?.datapack;
        Some(coordinate_scale(datapack, dimension.dimension_type()))
    }

    fn teleport_to_dimension(
        &mut self,
        player: PlayerId,
        dimension: &str,
        position: [f64; 3],
    ) -> bool {
        let Some(dimension) = crate::switch::dimension_named(dimension) else {
            return false;
        };
        let presence = self.presence(player);
        let Some(pose) = presence.pose.filter(|_| presence.alive) else {
            return false;
        };
        let Some(known) = self.player_mut(player) else {
            return false;
        };
        let place = crate::player_nbt::Place::standing(dimension, position, pose.yaw, pose.pitch);
        if !known.teleport_into(place) {
            return false;
        }
        let travelled = known.dimension == dimension;
        if let Some(pose) = self
            .presence_mut(player)
            .and_then(|presence| presence.pose.as_mut())
            .filter(|_| travelled)
        {
            pose.feet = position;
        }
        true
    }

    fn loaded(&self, dimension: &str, pos: [i32; 3]) -> Loaded {
        level_of(self.levels, dimension).map_or(Loaded::Unloaded, |level| {
            crate::command_blocks::loaded_at(&level.terrain.scene, pos)
        })
    }

    fn edit_blocks(&mut self, source: PlayerId, edit: &BlockEdit) -> Answer<i32> {
        let Some(entities) = loaded_dimension(self.levels, &edit.dimension)
            .and_then(|dimension| self.levels.get_mut(&dimension))
            .and_then(|level| level.entities.as_mut())
        else {
            return Answer::Now(0);
        };
        let edit = edit.clone();
        entities.run_job(Box::new(move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
            let count = minecraft_commands::apply_edit(&edit, &mut SimBlocks(sim));
            Box::new(CommandJob::Edit {
                source,
                edit,
                count,
            })
        }));
        Answer::Later
    }

    fn set_respawn(&mut self, player: PlayerId, respawn: &minecraft_commands::Respawn) -> bool {
        let Some(dimension) = Dimension::parse(&respawn.dimension) else {
            return false;
        };
        let Some(known) = self.player_mut(player) else {
            return false;
        };
        let [x, y, z] = respawn.pos;
        let point = crate::player_nbt::Respawn {
            at: crate::player_nbt::GlobalPos {
                dimension,
                pos: (x, y, z),
            },
            yaw: respawn.yaw,
            pitch: respawn.pitch,
            forced: respawn.forced,
        };
        if known.respawn != Some(point) {
            known.set_respawn(Some(point));
            self.respawn_changed.push(player);
        }
        true
    }

    fn locate(&mut self, source: PlayerId, request: &LocateRequest) -> Answer<Option<Located>> {
        let Some(dimension) = crate::switch::dimension_named(&request.dimension) else {
            return Answer::Now(None);
        };
        let Some(level) = self.levels.get_mut(&dimension) else {
            self.waiting_locates
                .push((dimension, source, request.clone()));
            return Answer::Later;
        };
        let Some(entities) = level.entities.as_mut() else {
            return Answer::Now(None);
        };
        entities.run_job(locate_job(source, request.clone(), &level.terrain.stream));
        Answer::Later
    }

    fn kit_line(&self) -> Option<String> {
        let key = minecraft_commands::kit::KIT_KEY;
        match &self.env_file {
            Some(path) => minecraft_commands::kit::read_env_file(path, key),
            None => std::env::var(key).ok(),
        }
    }

    fn kit_entries(&self, line: &str) -> Vec<(ItemArg, u32)> {
        minecraft_terrain::crafting::kit(line)
            .into_iter()
            .map(|(entry, count)| {
                let (id, enchantments) = minecraft_terrain::crafting::kit_enchantments(&entry);
                let (id, potion) = minecraft_terrain::crafting::kit_potion(id);
                let item = ItemArg {
                    id: id.to_owned(),
                    enchantments,
                    potion,
                    at: 0,
                };
                (item, count)
            })
            .collect()
    }

    fn level_up(&mut self, player: PlayerId) {
        let feet = self.presence(player).pose.map(|pose| pose.feet);
        let Some((player, entities)) = self.player_in_level(player) else {
            return;
        };
        let chime = player.survival.level_up_chime();
        if let (Some(volume), Some(entities), Some(feet)) = (chime, entities, feet) {
            entities.sounds.push((
                LEVEL_UP_SOUND.to_owned(),
                glam::DVec3::from_array(feet),
                volume,
                1.0,
            ));
        }
    }
}

pub fn locate_job(
    source: PlayerId,
    request: LocateRequest,
    stream: &TerrainStream,
) -> minecraft_terrain::server::Job {
    let worldgen = stream.world_gen();
    Box::new(move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
        let deadline = std::time::Instant::now() + LOCATE_TIME_CAP;
        let [x, y, z] = request.origin;
        let search = match request.kind {
            LocateKind::Structure => minecraft_terrain::locate::nearest_structure(
                &worldgen,
                &request.targets,
                request.origin,
                minecraft_terrain::locate::STRUCTURE_RADIUS,
                deadline,
            ),
            LocateKind::Biome => minecraft_terrain::locate::nearest_biome(
                &worldgen,
                &request.targets,
                request.origin,
                minecraft_terrain::locate::BIOME_RADIUS,
                deadline,
            ),
            LocateKind::Poi => sim
                .command_closest_poi(
                    &request.targets,
                    (x, y, z),
                    minecraft_commands::locate::POI_SEARCH_RADIUS,
                )
                .map_or(Search::NotFound, |((x, y, z), id)| {
                    Search::Found([x, y, z], id)
                }),
        };
        Box::new(CommandJob::Located {
            source,
            request,
            search,
        })
    })
}

fn held_enchantments(stack: &ItemStack) -> Vec<String> {
    let key = if stack.id == "minecraft:enchanted_book" {
        "minecraft:stored_enchantments"
    } else {
        "minecraft:enchantments"
    };
    stack
        .components
        .as_ref()
        .and_then(|components| components.get(key))
        .and_then(serde_json::Value::as_object)
        .map_or_else(Vec::new, |map| map.keys().cloned().collect())
}

pub fn loaded_dimension(levels: &BTreeMap<Dimension, Level>, id: &str) -> Option<Dimension> {
    levels
        .keys()
        .copied()
        .find(|dimension| dimension.dimension_type() == id)
}

pub fn level_of<'l>(levels: &'l BTreeMap<Dimension, Level>, id: &str) -> Option<&'l Level> {
    levels.get(&loaded_dimension(levels, id)?)
}

pub fn refresh_hints(
    hints: &mut CommandHints,
    chat: &CommandServer,
    dimensions: impl Iterator<Item = Dimension>,
    aim: Option<([f64; 3], [i32; 3])>,
) {
    let registries = chat.registries();
    if hints.items.len() != registries.items.len()
        || hints.enchantments.len() != registries.enchantments.len()
        || hints.blocks.len() != chat.data().blocks.len()
    {
        *hints = CommandHints::from_server(chat);
    }
    let players = chat.player_names();
    if hints.players != players {
        hints.players = players;
    }
    let dimensions: Vec<String> = dimensions
        .map(|dimension| dimension.dimension_type().to_owned())
        .collect();
    if hints.dimensions != dimensions {
        hints.dimensions = dimensions;
    }
    hints.hit = aim.map(|(point, _)| point);
    hints.hit_block = aim.map(|(_, block)| block);
}

pub fn coordinate_scale(datapack: &minecraftoss_core::datapack::DataPack, id: &str) -> f64 {
    minecraftoss_core::Identifier::parse(id)
        .and_then(|id| datapack.read_json("dimension_type", &id))
        .ok()
        .and_then(|json| json["coordinate_scale"].as_f64())
        .unwrap_or(1.0)
}

pub enum CommandJob {
    Edit {
        source: PlayerId,
        edit: BlockEdit,
        count: i32,
    },
    Located {
        source: PlayerId,
        request: LocateRequest,
        search: Search,
    },
}

fn located(request: &LocateRequest, search: Search) -> Option<Located> {
    match search {
        Search::Found(pos, id) => Some(Located { pos, id }),
        Search::NotFound => None,
        Search::TimedOut => {
            diag::warn!(
                World,
                "Minecraft /locate {:?} {} gave up after {} s",
                request.kind,
                request.printable,
                LOCATE_TIME_CAP.as_secs()
            );
            None
        }
    }
}

pub fn command_data(stream: &TerrainStream) -> CommandData {
    let worldgen = stream.world_gen();
    let registries = stream.states.registries();
    let blocks = registries
        .blocks
        .blocks()
        .map(|(_, block)| {
            let properties = block
                .properties()
                .iter()
                .map(|property| {
                    let values = property.values.iter().map(|value| value.to_string());
                    (property.name.to_string(), values.collect())
                })
                .collect();
            (block.name.to_string(), properties)
        })
        .collect();
    let tags = |registry: &str| minecraft_terrain::locate::tags(&worldgen, registry);
    CommandData {
        item_tags: tags("item"),
        blocks,
        block_tags: tags("block"),
        structures: minecraft_terrain::locate::structure_names(&worldgen),
        structure_tags: tags("worldgen/structure"),
        biomes: minecraft_terrain::locate::biome_names(&worldgen),
        biome_tags: tags("worldgen/biome"),
        poi_types: minecraftoss_entities::poi::PoiType::ALL
            .iter()
            .map(|kind| kind.id().to_owned())
            .collect(),
        poi_tags: tags("point_of_interest_type"),
    }
}

pub fn pump(
    from: PlayerId,
    sent: &mut Vec<String>,
    server: &mut CommandServer,
    world: &mut dyn CommandWorld,
) -> Vec<Clientbound> {
    let mut delivered = Vec::new();
    for line in std::mem::take(sent) {
        let Some(message) = Serverbound::from_input(&line) else {
            continue;
        };
        let level = server.level(from);
        delivered.extend(
            server
                .handle(from, message, world)
                .into_iter()
                .filter(|out| out.to.reaches(from, level))
                .map(|out| out.message),
        );
    }
    delivered
}

pub fn take_jobs_of(
    jobs: &mut Vec<Box<dyn Any + Send>>,
    source: PlayerId,
) -> Vec<Box<dyn Any + Send>> {
    let (taken, kept) = std::mem::take(jobs).into_iter().partition(|job| {
        job.downcast_ref::<CommandJob>().is_none_or(|job| match job {
            CommandJob::Edit { source: from, .. } | CommandJob::Located { source: from, .. } => {
                *from == source
            }
        })
    });
    *jobs = kept;
    taken
}

pub fn answer_jobs(
    chat: &CommandServer,
    rules: &GameRules,
    to: PlayerId,
    finished: Vec<Box<dyn Any + Send>>,
) -> Vec<Clientbound> {
    let level = chat.level(to);
    let mut delivered = Vec::new();
    for job in finished {
        let Ok(job) = job.downcast::<CommandJob>() else {
            continue;
        };
        let out = match *job {
            CommandJob::Edit {
                source,
                edit,
                count,
            } => chat.reply(
                source,
                minecraft_commands::blocks::edit_answer(&edit, count),
                true,
                rules,
            ),
            CommandJob::Located {
                source,
                request,
                search,
            } => {
                let found = located(&request, search);
                let answer = minecraft_commands::locate_answer(&request, found.as_ref());
                chat.reply(source, answer.map(|(text, _)| text), false, rules)
            }
        };
        delivered.extend(
            out.into_iter()
                .filter(|out| out.to.reaches(to, level))
                .map(|out| out.message),
        );
    }
    delivered
}

#[derive(Default)]
pub struct Handled {
    pub delivered: Vec<Clientbound>,
    pub teleports: Vec<(PlayerUuid, [f64; 3])>,
}

pub fn run(
    globals: &mut Globals,
    levels: &mut BTreeMap<Dimension, Level>,
    players: &mut BTreeMap<PlayerUuid, Player>,
    from: &PlayerUuid,
    sent: &mut Vec<String>,
    presence: Vec<(PlayerUuid, Presence)>,
) -> Handled {
    let Some(id) = players.get(from).map(|player| player.id) else {
        return Handled::default();
    };
    let mut host = Host {
        seed: globals.seed,
        rules: &mut globals.game_rules,
        difficulty: &mut globals.difficulty,
        day: &mut globals.day,
        end_clock: &mut globals.end_clock,
        save: globals.save.as_mut(),
        data: &globals.data,
        registries: Some(&globals.registries),
        levels: &mut *levels,
        waiting_locates: &mut globals.waiting_locates,
        players: &mut *players,
        presence,
        env_file: if sent.is_empty() { None } else { env_file() },
        rules_changed: false,
        mode_changed: Vec::new(),
        respawn_changed: Vec::new(),
        teleports: Vec::new(),
    };
    let mut delivered = pump(id, sent, &mut globals.chat, &mut host);
    let rules_changed = host.rules_changed;
    let mode_changed = std::mem::take(&mut host.mode_changed);
    let respawn_changed = std::mem::take(&mut host.respawn_changed);
    let teleports = std::mem::take(&mut host.teleports);
    drop(host);
    let mut finished = Vec::new();
    for entities in levels.values_mut().filter_map(|level| level.entities.as_mut()) {
        entities.poll_jobs();
        finished.extend(take_jobs_of(&mut entities.command_jobs, id));
    }
    delivered.extend(answer_jobs(&globals.chat, &globals.game_rules, id, finished));
    if rules_changed {
        for entities in levels.values_mut().filter_map(|level| level.entities.as_mut()) {
            entities.set_game_rules(&globals.game_rules);
        }
    }
    for warning in globals.chat.take_warnings() {
        diag::warn!(World, "{warning}");
    }
    for player in players
        .values()
        .filter(|player| mode_changed.contains(&player.id) || respawn_changed.contains(&player.id))
    {
        crate::server::save_player(globals, player);
    }
    let teleports = teleports
        .into_iter()
        .filter_map(|(id, to)| {
            let player = players.values().find(|player| player.id == id)?;
            Some((player.uuid.clone(), to))
        })
        .collect();
    Handled {
        delivered,
        teleports,
    }
}

pub fn death_message(line: String) -> Clientbound {
    Clientbound::System {
        text: Text::literal(line),
        overlay: false,
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    use minecraft_terrain::game_mode::PlayerMode;

    pub(crate) const FRIEND: PlayerId = PlayerId(1);

    pub(crate) struct Parts {
        pub(crate) rules: GameRules,
        pub(crate) difficulty: Difficulty,
        pub(crate) day: DayCycle,
        pub(crate) end_clock: DayCycle,
        pub(crate) data: crate::data::GameData,
        pub(crate) registries: Option<std::sync::Arc<minecraftoss_core::Registries>>,
        pub(crate) levels: BTreeMap<Dimension, Level>,
        pub(crate) waiting_locates: Vec<(Dimension, PlayerId, LocateRequest)>,
        pub(crate) players: BTreeMap<PlayerUuid, Player>,
        pub(crate) presence: Vec<(PlayerUuid, Presence)>,
        pub(crate) server: CommandServer,
        pub(crate) env_file: Option<PathBuf>,
    }

    impl Parts {
        pub(crate) fn new() -> Self {
            let mut parts = Self {
                rules: GameRules::default(),
                difficulty: Difficulty::Normal,
                day: DayCycle::default(),
                end_clock: DayCycle::default(),
                data: crate::data::GameData::default(),
                registries: None,
                levels: BTreeMap::new(),
                waiting_locates: Vec::new(),
                players: BTreeMap::new(),
                presence: Vec::new(),
                server: CommandServer::new(HOST),
                env_file: None,
            };
            parts.join(HOST, "Steve", [4.5, 70.0, -2.5]);
            parts
        }

        pub(crate) fn join(&mut self, id: PlayerId, name: &str, feet: [f64; 3]) -> PlayerUuid {
            self.server.join(id, name);
            let mut survival = crate::survival::Survival::default();
            survival.observe(100, 100);
            let player = Player::new(
                self.server.profile(id).unwrap(),
                PlayerMode::default(),
                survival,
                Inventory::default(),
            );
            let uuid = player.uuid.clone();
            let pose = Pose {
                feet,
                yaw: 0.0,
                pitch: 0.0,
            };
            self.presence.push((
                uuid.clone(),
                Presence {
                    pose: Some(pose),
                    alive: true,
                },
            ));
            self.players.insert(uuid.clone(), player);
            uuid
        }

        pub(crate) fn player(&mut self, id: PlayerId) -> &mut Player {
            self.players
                .values_mut()
                .find(|player| player.id == id)
                .unwrap()
        }

        pub(crate) fn host(&mut self) -> Host<'_> {
            Host {
                seed: 42,
                rules: &mut self.rules,
                difficulty: &mut self.difficulty,
                day: &mut self.day,
                end_clock: &mut self.end_clock,
                save: None,
                data: &self.data,
                registries: self.registries.as_deref(),
                levels: &mut self.levels,
                waiting_locates: &mut self.waiting_locates,
                players: &mut self.players,
                presence: self.presence.clone(),
                env_file: self.env_file.clone(),
                rules_changed: false,
                mode_changed: Vec::new(),
                respawn_changed: Vec::new(),
                teleports: Vec::new(),
            }
        }

        pub(crate) fn send(
            &mut self,
            from: PlayerId,
            lines: &[&str],
        ) -> (Vec<String>, Vec<(PlayerId, [f64; 3])>) {
            let mut sent: Vec<String> = lines.iter().map(|line| (*line).to_owned()).collect();
            let mut server = std::mem::take(&mut self.server);
            let mut host = self.host();
            let delivered = pump(from, &mut sent, &mut server, &mut host);
            let teleports = std::mem::take(&mut host.teleports);
            drop(host);
            self.server = server;
            (delivered.iter().map(line).collect(), teleports)
        }
    }

    pub(crate) fn lang(key: &str) -> Option<String> {
        let text = match key {
            "chat.square_brackets" => "[%s]",
            "chat.coordinates" => "%s, %s, %s",
            "commands.kill.success.single" => "Killed %s",
            "commands.kill.success.multiple" => "Killed %s entities",
            "commands.gamemode.success.self" => "Set own game mode to %s",
            "commands.gamemode.success.other" => "Set %s's game mode to %s",
            "gameMode.creative" => "Creative Mode",
            "commands.teleport.success.location.single" => "Teleported %s to %s, %s, %s",
            "commands.effect.give.success.single" => "Applied effect %s to %s",
            "argument.entity.notfound.player" => "No player was found",
            "effect.minecraft.regeneration" => "Regeneration",
            "commands.clear.test.single" => "Found %s matching item(s) on player %s",
            "commands.clear.success.single" => "Removed %s item(s) from player %s",
            "clear.failed.single" => "No items were found on player %s",
            "commands.experience.add.levels.success.single" => "Gave %s experience levels to %s",
            "commands.experience.add.points.success.single" => "Gave %s experience points to %s",
            "commands.experience.query.levels" => "%s has %s experience levels",
            "commands.experience.set.points.invalid" => {
                "Cannot set experience points above the maximum points for the player's current level"
            }
            "commands.enchant.success.single" => "Applied enchantment %s to %s's item",
            "commands.enchant.failed.incompatible" => "%s cannot support that enchantment",
            "commands.enchant.failed.itemless" => "%s is not holding any item",
            "commands.enchant.failed.level" => {
                "%s is higher than the maximum level of %s supported by that enchantment"
            }
            "enchantment.minecraft.sharpness" => "Sharpness",
            "enchantment.minecraft.unbreaking" => "Unbreaking",
            "enchantment.minecraft.binding_curse" => "Curse of Binding",
            "enchantment.level.1" => "I",
            "enchantment.level.3" => "III",
            "enchantment.level.5" => "V",
            "item.minecraft.diamond_sword" => "Diamond Sword",
            "item.minecraft.stick" => "Stick",
            "chat.type.admin" => "[%s: %s]",
            "commands.setblock.success" => "Changed the block at %s, %s, %s",
            "commands.setblock.failed" => "Could not set the block",
            "commands.fill.success" => "Successfully filled %s block(s)",
            "commands.fill.failed" => "No blocks were filled",
            "argument.pos.unloaded" => "That position is not loaded",
            "argument.dimension.invalid" => "Unknown dimension '%s'",
            "commands.give.success.single" => "Gave %s %s to %s",
            "commands.spawnpoint.success.single" => {
                "Set spawn point to %s, %s, %s [%s] in %s for %s"
            }
            "commands.spawnpoint.success.multiple" => {
                "Set spawn point to %s, %s, %s [%s] in %s for %s players"
            }
            "commands.seed.success" => "Seed: %s",
            "commands.time.set.absolute" => "Set %s to %s tick(s)",
            "commands.time.pause" => "Paused clock %s",
            "commands.time.query.absolute" => "Clock %s is at %s tick(s)",
            "commands.time.no_default_clock" => "There is no default clock in dimension %s",
            "block.minecraft.stone" => "Stone",
            "block.minecraft.torch" => "Torch",
            "commands.locate.structure.success" => "The nearest %s is at %s (%s blocks away)",
            "commands.locate.biome.success" => "The nearest %s is at %s (%s blocks away)",
            "commands.locate.structure.not_found" => {
                "Could not find a structure of type \"%s\" nearby"
            }
            "commands.locate.poi.not_found" => {
                "Could not find a point of interest of type \"%s\" within a reasonable distance"
            }
            _ => return None,
        };
        Some(text.to_owned())
    }

    pub(crate) fn line(message: &Clientbound) -> String {
        message
            .chat_text()
            .map_or_else(String::new, |text| text.resolve(&lang).text)
    }

    pub(crate) fn test_stream() -> Option<TerrainStream> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = minecraftoss_core::Registries::load(&paths).ok()?;
        TerrainStream::for_dimension(
            std::sync::Arc::new(registries),
            0,
            2,
            Dimension::Overworld,
            None,
        )
        .ok()
    }

    #[test]
    fn clear_takes_items_from_the_named_player_like_vanilla() {
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        parts.server.data_mut().item_tags.insert(
            "minecraft:logs".into(),
            vec!["minecraft:oak_log".into(), "minecraft:birch_log".into()],
        );
        let steve = &mut parts.player(HOST).inventory;
        steve.slots[0] = Some(ItemStack::new("minecraft:stone", 64));
        steve.slots[1] = Some(ItemStack::new("minecraft:stone", 30));
        steve.slots[2] = Some(ItemStack::new("minecraft:oak_log", 10));
        steve.slots[36] = Some(ItemStack::new("minecraft:iron_boots", 1));
        steve.cursor = Some(ItemStack::new("minecraft:stone", 1));
        parts.player(FRIEND).inventory.slots[4] = Some(ItemStack::new("minecraft:stone", 5));
        let (lines, _) = parts.send(
            HOST,
            &[
                "/clear @s minecraft:stone 0",
                "/clear @s stone 70",
                "/clear @s #minecraft:logs",
                "/clear Alex",
                "/clear Alex",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "Found 95 matching item(s) on player Steve",
                "Removed 70 item(s) from player Steve",
                "Removed 10 item(s) from player Steve",
                "Removed 5 item(s) from player Alex",
                "No items were found on player Alex",
            ]
        );
        let steve = &parts.player(HOST).inventory;
        let left: Vec<(usize, &str, u8)> = steve
            .slots
            .iter()
            .enumerate()
            .filter_map(|(slot, stack)| stack.as_ref().map(|s| (slot, s.id.as_str(), s.count)))
            .collect();
        assert_eq!(
            left,
            vec![(1, "minecraft:stone", 24), (36, "minecraft:iron_boots", 1)]
        );
        assert_eq!(steve.cursor.as_ref().map(|s| s.count), Some(1));
        assert_eq!(parts.host().clear(PlayerId(7), &ItemFilter::Any, -1), None);
    }

    #[test]
    fn experience_commands_fill_the_named_players_bar() {
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let (lines, _) = parts.send(
            HOST,
            &[
                "/xp add Alex 5 levels",
                "/xp query Alex levels",
                "/xp add @s 10",
                "/xp query @s levels",
                "/xp set Alex 30 points",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "Gave 5 experience levels to Alex",
                "Alex has 5 experience levels",
                "Gave 10 experience points to Steve",
                "Steve has 1 experience levels",
                "Cannot set experience points above the maximum points for the player's current level",
            ]
        );
        let alex = &mut parts.player(FRIEND).survival;
        assert_eq!(
            (alex.status.experience_level, alex.status.total_experience),
            (5, 0)
        );
        assert_eq!(alex.level_up_chime(), None);
        let steve = &mut parts.player(HOST).survival;
        assert_eq!(
            (steve.status.experience_level, steve.status.total_experience),
            (1, 10)
        );
        assert!((steve.status.experience_progress - 3.0 / 9.0).abs() < 1e-5);
        assert!(steve.level_up_chime().is_some());
        let mut host = parts.host();
        assert!(host.experience(PlayerId(7)).is_none());
        assert!(!host.set_experience(PlayerId(7), Experience::default()));
        assert!(host.set_experience(
            HOST,
            Experience {
                level: -3,
                progress: 0.0,
                total: -1
            }
        ));
        assert_eq!(host.experience(HOST), Some(Experience::default()));
    }

    #[test]
    fn spawnpoint_sets_the_named_players_respawn_point_and_saves_it() {
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let (lines, _) = parts.send(HOST, &["/spawnpoint Alex 10 64 -4 90 0", "/spawnpoint"]);
        assert_eq!(
            lines,
            vec![
                "Set spawn point to 10, 64, -4 [90.0] in 0.0 for minecraft:overworld",
                "Set spawn point to 4, 70, -3 [0.0] in 0.0 for minecraft:overworld",
            ]
        );
        let point = |pos, yaw| crate::player_nbt::Respawn {
            at: crate::player_nbt::GlobalPos {
                dimension: Dimension::Overworld,
                pos,
            },
            yaw,
            pitch: 0.0,
            forced: true,
        };
        assert_eq!(parts.player(FRIEND).respawn, Some(point((10, 64, -4), 90.0)));
        assert_eq!(parts.player(FRIEND).data().respawn, Some(point((10, 64, -4), 90.0)));
        assert_eq!(parts.player(HOST).respawn, Some(point((4, 70, -3), 0.0)));
        let alex = parts.player(FRIEND);
        alex.observe_life(playerstate_iw4::PM_TYPE_DEAD);
        assert!(alex.observe_life(0));
        assert!(!alex.arrive_at_respawn(&[Dimension::Overworld], |_| Some(true), |_| None));
        assert_eq!(
            alex.arrival.map(|place| (place.feet, place.yaw)),
            Some(([10.5, 64.0, -3.5], 90.0))
        );
        let mut server = std::mem::take(&mut parts.server);
        let mut host = parts.host();
        let mut sent = vec![
            "/spawnpoint Alex 10 64 -4 90 0".to_owned(),
            "/spawnpoint Alex 10 64 -4 -90 0".to_owned(),
        ];
        pump(HOST, &mut sent, &mut server, &mut host);
        assert_eq!(host.respawn_changed, vec![FRIEND]);
        let moon = minecraft_commands::Respawn {
            dimension: "minecraft:moon".into(),
            pos: [0, 64, 0],
            yaw: 0.0,
            pitch: 0.0,
            forced: true,
        };
        assert!(!host.set_respawn(HOST, &moon));
        let nowhere = minecraft_commands::Respawn {
            dimension: "minecraft:overworld".into(),
            ..moon
        };
        assert!(!host.set_respawn(PlayerId(7), &nowhere));
        assert_eq!(host.respawn_changed, vec![FRIEND]);
    }

    fn enchantment_book() -> minecraftoss_entities::enchant::Enchantments {
        use serde_json::json;
        let sword = "minecraft:diamond_sword";
        let definitions = vec![
            (
                "minecraft:sharpness".to_owned(),
                json!({"max_level": 5, "supported_items": [sword], "exclusive_set": ["minecraft:smite"]}),
            ),
            (
                "minecraft:smite".to_owned(),
                json!({"max_level": 5, "supported_items": [sword], "exclusive_set": ["minecraft:sharpness"]}),
            ),
            (
                "minecraft:unbreaking".to_owned(),
                json!({"max_level": 3, "supported_items": [sword, "minecraft:stick"]}),
            ),
            (
                "minecraft:binding_curse".to_owned(),
                json!({"max_level": 1, "supported_items": ["minecraft:iron_helmet"]}),
            ),
        ];
        let tags = HashMap::from([(
            "minecraft:curse".to_owned(),
            json!({"values": ["minecraft:binding_curse"]}),
        )]);
        minecraftoss_entities::enchant::Enchantments::new(
            definitions,
            &tags,
            &HashMap::new(),
            HashMap::new(),
        )
    }

    #[test]
    fn enchant_upgrades_the_named_players_held_item() {
        let mut parts = Parts::new();
        parts.data.enchantments = Some(std::sync::Arc::new(enchantment_book()));
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        parts.player(HOST).inventory.slots[0] =
            Some(ItemStack::new("minecraft:diamond_sword", 1));
        parts.player(FRIEND).inventory.slots[0] = Some(ItemStack::new("minecraft:stick", 1));
        parts.player(FRIEND).inventory.slots[3] =
            Some(ItemStack::new("minecraft:diamond_sword", 1));
        let (lines, _) = parts.send(
            HOST,
            &[
                "/enchant @s sharpness 5",
                "/enchant @s sharpness 6",
                "/enchant @s minecraft:smite",
                "/enchant @s sharpness 2",
                "/enchant @s unbreaking 3",
                "/enchant Alex binding_curse",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "Applied enchantment Sharpness V to Steve's item",
                "6 is higher than the maximum level of 5 supported by that enchantment",
                "Diamond Sword cannot support that enchantment",
                "Diamond Sword cannot support that enchantment",
                "Applied enchantment Unbreaking III to Steve's item",
                "Stick cannot support that enchantment",
            ]
        );
        let sword = parts.player(HOST).inventory.slots[0].clone().unwrap();
        assert_eq!(
            sword.components,
            Some(serde_json::json!({"minecraft:enchantments": {
                "minecraft:sharpness": 5,
                "minecraft:unbreaking": 3
            }}))
        );
        parts.player(FRIEND).selected = 3;
        parts.player(HOST).selected = 8;
        let (lines, _) = parts.send(HOST, &["/enchant Alex unbreaking", "/enchant @s unbreaking"]);
        assert_eq!(
            lines,
            vec![
                "Applied enchantment Unbreaking I to Alex's item",
                "Steve is not holding any item",
            ]
        );
        let host = parts.host();
        assert_eq!(
            host.enchantment("minecraft:binding_curse"),
            Some(EnchantmentInfo {
                max_level: 1,
                curse: true
            })
        );
        assert_eq!(host.enchantment("minecraft:nope"), None);
        assert_eq!(
            held_enchantments(&ItemStack {
                components: Some(serde_json::json!({"minecraft:stored_enchantments": {"minecraft:mending": 1}})),
                ..ItemStack::new("minecraft:enchanted_book", 1)
            }),
            vec!["minecraft:mending".to_owned()]
        );
    }

    #[test]
    fn block_edits_answer_once_the_level_has_run_them() {
        use minecraft_commands::blocks::{BlockInput, EditKind, FillMode};
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let (lines, _) = parts.send(
            HOST,
            &[
                "/setblock ~ ~ ~ stone",
                "/fill ~ ~ ~ ~1 ~1 ~1 stone",
                "/execute in minecraft:the_nether run setblock 0 0 0 stone",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "That position is not loaded",
                "That position is not loaded",
                "Unknown dimension 'minecraft:the_nether'",
            ]
        );
        let job = |source: PlayerId, kind: EditKind, count: i32| -> Box<dyn Any + Send> {
            Box::new(CommandJob::Edit {
                source,
                edit: BlockEdit {
                    kind,
                    dimension: minecraft_commands::OVERWORLD.to_owned(),
                    from: [1, 2, 3],
                    to: [4, 5, 6],
                    block: BlockInput::default(),
                    mode: FillMode::Replace,
                    filter: None,
                    strict: false,
                },
                count,
            })
        };
        let answers = answer_jobs(
            &parts.server,
            &parts.rules,
            HOST,
            vec![
                job(HOST, EditKind::SetBlock, 1),
                job(HOST, EditKind::SetBlock, 0),
                job(HOST, EditKind::Fill, 98),
                job(HOST, EditKind::Fill, 0),
                Box::new(5_i32),
                job(FRIEND, EditKind::SetBlock, 1),
                job(FRIEND, EditKind::Fill, 0),
            ],
        );
        assert_eq!(
            answers.iter().map(line).collect::<Vec<_>>(),
            vec![
                "Changed the block at 1, 2, 3",
                "Could not set the block",
                "Successfully filled 98 block(s)",
                "No blocks were filled",
                "[Alex: Changed the block at 1, 2, 3]",
            ]
        );
        let host = parts.host();
        assert_eq!(host.dimension(HOST), "minecraft:overworld");
        assert_eq!(host.loaded("minecraft:overworld", [0, 64, 0]), Loaded::Unloaded);
    }

    #[test]
    fn each_player_takes_only_the_answers_to_their_own_commands() {
        use minecraft_commands::blocks::{BlockInput, EditKind, FillMode};
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let edit = |source: PlayerId, count: i32| -> Box<dyn Any + Send> {
            Box::new(CommandJob::Edit {
                source,
                edit: BlockEdit {
                    kind: EditKind::SetBlock,
                    dimension: minecraft_commands::OVERWORLD.to_owned(),
                    from: [1, 2, 3],
                    to: [1, 2, 3],
                    block: BlockInput::default(),
                    mode: FillMode::Replace,
                    filter: None,
                    strict: false,
                },
                count,
            })
        };
        let located = |source: PlayerId| -> Box<dyn Any + Send> {
            Box::new(CommandJob::Located {
                source,
                request: LocateRequest {
                    kind: LocateKind::Structure,
                    printable: "minecraft:fortress".to_owned(),
                    tag: false,
                    targets: Vec::new(),
                    dimension: minecraft_commands::OVERWORLD.to_owned(),
                    origin: [0, 64, 0],
                },
                search: Search::NotFound,
            })
        };
        let mut jobs = vec![edit(HOST, 1), edit(FRIEND, 1), located(FRIEND), edit(HOST, 0)];
        let steve = take_jobs_of(&mut jobs, HOST);
        assert_eq!(steve.len(), 2);
        assert_eq!(jobs.len(), 2);
        let alex = take_jobs_of(&mut jobs, FRIEND);
        assert!(jobs.is_empty());
        let answers = |to: PlayerId, finished| {
            answer_jobs(&parts.server, &parts.rules, to, finished)
                .iter()
                .map(line)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            answers(HOST, steve),
            vec!["Changed the block at 1, 2, 3", "Could not set the block"]
        );
        assert_eq!(
            answers(FRIEND, alex),
            vec![
                "Changed the block at 1, 2, 3",
                "Could not find a structure of type \"minecraft:fortress\" nearby",
            ]
        );
        let mut other: Vec<Box<dyn Any + Send>> = vec![Box::new(5_i32)];
        assert_eq!(take_jobs_of(&mut other, FRIEND).len(), 1);
        assert!(other.is_empty());
    }

    pub(crate) fn test_level(stream: TerrainStream) -> Option<Level> {
        let registries = stream.states.registries().clone();
        let environment = minecraft_terrain::environment::DimensionEnvironment::load(
            &registries,
            Dimension::Overworld.dimension_type(),
        )
        .ok()?;
        let rules =
            crate::dimension::DimensionRules::load(&registries, Dimension::Overworld).ok()?;
        let scene = minecraft_terrain::scene::HandcraftedScene::streamed(stream.states.clone());
        Some(Level {
            dimension: Dimension::Overworld,
            entities: None,
            containers: Default::default(),
            terrain: crate::level::Terrain {
                stream,
                scene,
                environment,
                rules,
            },
            openers: Default::default(),
            empty_ticks: 0,
        })
    }

    #[test]
    fn execute_in_finds_every_real_level_open_or_not() {
        let mut parts = Parts::new();
        let (lines, _) = parts.send(HOST, &["/execute in minecraft:overworld run seed"]);
        assert_eq!(lines, vec!["Unknown dimension 'minecraft:overworld'"]);
        assert_eq!(loaded_dimension(&parts.levels, "minecraft:overworld"), None);
        let Some(level) = test_stream().and_then(test_level) else {
            return;
        };
        parts.registries = Some(level.terrain.stream.states.registries().clone());
        let (lines, _) = parts.send(
            HOST,
            &[
                "/execute in minecraft:overworld run seed",
                "/execute in minecraft:the_nether run seed",
                "/execute in the_end run seed",
                "/execute in minecraft:the_moon run seed",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "Seed: [42]",
                "Seed: [42]",
                "Seed: [42]",
                "Unknown dimension 'minecraft:the_moon'",
            ]
        );
        assert!(
            parts.levels.is_empty(),
            "looking a level up does not open it"
        );
        parts.levels.insert(Dimension::Overworld, level);
        let datapack = parts.levels[&Dimension::Overworld]
            .terrain
            .stream
            .states
            .registries()
            .datapack
            .clone();
        assert_eq!(coordinate_scale(&datapack, "minecraft:the_nether"), 8.0);
        assert_eq!(coordinate_scale(&datapack, "minecraft:the_end"), 1.0);
        assert_eq!(coordinate_scale(&datapack, "minecraft:nope"), 1.0);
        let host = parts.host();
        assert_eq!(host.dimension_scale("minecraft:overworld"), Some(1.0));
        assert_eq!(host.dimension_scale("minecraft:the_nether"), Some(8.0));
        assert_eq!(host.dimension_scale("minecraft:the_end"), Some(1.0));
        assert_eq!(host.dimension_scale("minecraft:nope"), None);
        assert_eq!(
            host.loaded("minecraft:overworld", [0, 64, 0]),
            Loaded::Unloaded
        );
    }

    #[test]
    fn execute_in_teleports_the_player_into_another_dimension_through_the_switch() {
        let mut parts = Parts::new();
        parts.registries = minecraftoss_core::registries::DataPaths::discover()
            .ok()
            .and_then(|paths| minecraftoss_core::Registries::load(&paths).ok())
            .map(std::sync::Arc::new);
        if parts.registries.is_none() {
            return;
        }
        parts.presence[0].1.pose = Some(Pose {
            feet: [800.0, 70.0, -1600.0],
            yaw: 45.0,
            pitch: 10.0,
        });
        let (lines, teleports) =
            parts.send(HOST, &["/execute in minecraft:the_nether run tp ~ 128 ~"]);
        assert_eq!(
            lines,
            vec!["Teleported Steve to 100.000000, 128.000000, -200.000000"]
        );
        assert!(teleports.is_empty(), "no teleport inside the old level");
        let roof = crate::player_nbt::Place::standing(
            Dimension::Nether,
            [100.0, 128.0, -200.0],
            45.0,
            10.0,
        );
        let steve = parts.player(HOST);
        assert_eq!(steve.dimension, Dimension::Nether);
        assert_eq!(steve.arrival, Some(roof));
        let switch = steve.switching.expect("the switch started");
        assert_eq!(
            (switch.from, switch.to),
            (Dimension::Overworld, Dimension::Nether)
        );
        assert_eq!(switch.arrival, Some(roof));
        assert_eq!(switch.reason, crate::switch::LoadingReason::NetherPortal);
        parts.presence[0].1.pose = Some(Pose {
            feet: [100.0, 128.0, -200.0],
            yaw: 45.0,
            pitch: 10.0,
        });
        let (lines, teleports) =
            parts.send(HOST, &["/execute in minecraft:the_nether run tp ~ ~ ~"]);
        assert_eq!(
            lines,
            vec!["Teleported Steve to 100.000000, 128.000000, -200.000000"]
        );
        assert_eq!(
            teleports,
            vec![(HOST, [100.0, 128.0, -200.0])],
            "same level: a plain teleport"
        );
        parts.player(HOST).switching = None;
        parts.player(HOST).set_remote(true);
        let (lines, _) = parts.send(HOST, &["/execute in minecraft:overworld run tp ~ ~ ~"]);
        assert_eq!(
            lines,
            vec!["Teleported Steve to 800.000000, 128.000000, -1600.000000"]
        );
        let steve = parts.player(HOST);
        assert_eq!(
            steve.dimension,
            Dimension::Nether,
            "held during a remote killstreak"
        );
        assert!(steve.switching.is_none());
        assert_eq!(steve.held.map(|held| held.to), Some(Dimension::Overworld));
        steve.set_remote(false);
        assert_eq!(steve.dimension, Dimension::Overworld);
        parts.player(HOST).switching = None;
        parts.presence[0].1.alive = false;
        let (lines, _) = parts.send(HOST, &["/execute in minecraft:the_end run tp 100 50 0"]);
        assert!(lines.is_empty(), "nobody to move: {lines:?}");
        assert_eq!(parts.player(HOST).dimension, Dimension::Overworld);
        let host = parts.host();
        assert_eq!(host.dimension_scale("minecraft:the_nether"), Some(8.0));
        assert!(
            parts.levels.is_empty(),
            "the switch opens the level, not the command"
        );
    }

    #[test]
    fn time_follows_the_default_clock_of_the_player_s_dimension() {
        let mut parts = Parts::new();
        parts.day.set(1000.0);
        parts.player(HOST).dimension = Dimension::Nether;
        let (lines, _) = parts.send(HOST, &["/time set 5000", "/time pause"]);
        assert_eq!(
            lines,
            vec![
                "There is no default clock in dimension minecraft:the_nether",
                "There is no default clock in dimension minecraft:the_nether",
            ]
        );
        assert_eq!((parts.day.ticks, parts.end_clock.ticks), (1000.0, 1000.0));
        assert!(!parts.day.paused && !parts.end_clock.paused);
        parts.player(HOST).dimension = Dimension::End;
        let (lines, _) = parts.send(HOST, &["/time set 5000", "/time pause", "/time query time"]);
        assert_eq!(
            lines,
            vec![
                "Set minecraft:the_end to 5000 tick(s)",
                "Paused clock minecraft:the_end",
                "Clock minecraft:the_end is at 5000 tick(s)",
            ]
        );
        assert_eq!(
            (parts.end_clock.ticks, parts.end_clock.paused),
            (5000.0, true)
        );
        assert_eq!((parts.day.ticks, parts.day.paused), (1000.0, false));
        parts.player(HOST).dimension = Dimension::Overworld;
        let (lines, _) = parts.send(HOST, &["/time set 3000"]);
        assert_eq!(lines, vec!["Set minecraft:overworld to 3000 tick(s)"]);
        assert_eq!((parts.day.ticks, parts.end_clock.ticks), (3000.0, 5000.0));
        let host = parts.host();
        assert_eq!(
            host.default_clock("minecraft:overworld").as_deref(),
            Some("minecraft:overworld")
        );
        assert_eq!(host.default_clock("minecraft:the_nether"), None);
        assert_eq!(
            host.default_clock("minecraft:the_end").as_deref(),
            Some("minecraft:the_end")
        );
    }

    #[test]
    fn hints_follow_the_server_and_the_point_in_view() {
        let mut parts = Parts::new();
        let mut hints = CommandHints::default();
        let overworld = || [Dimension::Overworld].into_iter();
        refresh_hints(&mut hints, &parts.server, overworld(), None);
        assert_eq!(hints.players, vec!["Steve".to_owned()]);
        assert_eq!(hints.dimensions, vec!["minecraft:overworld".to_owned()]);
        assert_eq!((hints.hit, hints.hit_block), (None, None));
        parts.server.registries_mut().items = vec!["minecraft:stone".to_owned()];
        parts.server.registries_mut().enchantments = vec!["minecraft:sharpness".to_owned()];
        parts.server.data_mut().blocks.insert(
            "minecraft:oak_stairs".to_owned(),
            vec![("half".to_owned(), vec!["top".to_owned(), "bottom".to_owned()])],
        );
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let aim = ([3.5, 64.0, -2.125], [3, 63, -3]);
        refresh_hints(
            &mut hints,
            &parts.server,
            [Dimension::Overworld, Dimension::Nether].into_iter(),
            Some(aim),
        );
        assert_eq!(hints.items, vec!["minecraft:stone".to_owned()]);
        assert_eq!(hints.enchantments, vec!["minecraft:sharpness".to_owned()]);
        assert_eq!(hints.blocks_with_properties, vec!["minecraft:oak_stairs".to_owned()]);
        assert_eq!(hints.players, vec!["Steve".to_owned(), "Alex".to_owned()]);
        assert_eq!(
            hints.dimensions,
            vec![
                "minecraft:overworld".to_owned(),
                "minecraft:the_nether".to_owned()
            ]
        );
        assert_eq!(hints.hit, Some([3.5, 64.0, -2.125]));
        assert_eq!(hints.hit_block, Some([3, 63, -3]));
        let completion = minecraft_commands::complete("/tp ", 4, 4, &hints);
        assert!(
            completion
                .list
                .iter()
                .any(|suggestion| suggestion.text == "3.50 64.00 -2.13")
        );
        parts.server.leave(FRIEND);
        refresh_hints(&mut hints, &parts.server, overworld(), None);
        assert_eq!(hints.players, vec!["Steve".to_owned()]);
        assert_eq!(hints.items, vec!["minecraft:stone".to_owned()]);
        assert_eq!((hints.hit, hints.hit_block), (None, None));
    }

    #[test]
    fn kit_reads_the_env_file_again_and_gives_like_give() {
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [0.5, 64.0, 0.5]);
        let path = std::env::temp_dir().join(format!("iw4l-kit-{}.env", std::process::id()));
        let kit = "IW4L_GAMES=C:\\Games\r\nIW4L_MINECRAFT_KIT=stone*70,diamond_sword+sharpness=5\r\n";
        std::fs::write(&path, kit).unwrap();
        parts.env_file = Some(path.clone());
        let (lines, _) = parts.send(HOST, &["/kit", "/kit Alex"]);
        assert_eq!(
            lines,
            vec![
                "Gave 70 [Stone] to Steve",
                "Gave 1 [Diamond Sword] to Steve",
                "Gave 70 [Stone] to Alex",
                "Gave 1 [Diamond Sword] to Alex",
            ]
        );
        for id in [HOST, FRIEND] {
            let slots = &parts.player(id).inventory.slots;
            let stone: u32 = slots
                .iter()
                .flatten()
                .filter(|stack| stack.id == "minecraft:stone")
                .map(|stack| u32::from(stack.count))
                .sum();
            assert_eq!(stone, 70);
            let sword = slots
                .iter()
                .flatten()
                .find(|stack| stack.id == "minecraft:diamond_sword")
                .unwrap();
            let components = sword.components.as_ref().unwrap();
            assert_eq!(
                components["minecraft:enchantments"]["minecraft:sharpness"],
                5
            );
        }
        std::fs::write(&path, "IW4L_MINECRAFT_KIT=torch*8\n").unwrap();
        let (lines, _) = parts.send(HOST, &["/kit"]);
        assert_eq!(lines, vec!["Gave 8 [Torch] to Steve"]);
        std::fs::write(&path, "IW4L_GAMES=C:\\Games\n").unwrap();
        let (lines, _) = parts.send(HOST, &["/kit"]);
        assert_eq!(
            lines,
            vec!["No IW4L_MINECRAFT_KIT in the .env next to the game"]
        );
        std::fs::remove_file(&path).unwrap();
    }

    #[test]
    fn kit_gives_potions_and_colours_like_the_start_kit() {
        let mut parts = Parts::new();
        let path =
            std::env::temp_dir().join(format!("iw4l-kit-potions-{}.env", std::process::id()));
        let line = "potion@healing*2,tipped_arrow@minecraft:poison*4,leather_chestplate@red,splash_potion@,diamond_sword+sharpness=2@x*0";
        std::fs::write(&path, format!("IW4L_MINECRAFT_KIT={line}\n")).unwrap();
        parts.env_file = Some(path.clone());
        let (lines, _) = parts.send(HOST, &["/kit"]);
        std::fs::remove_file(&path).unwrap();
        assert_eq!(lines.len(), 4, "{lines:?}");
        let mut start = Inventory::default();
        minecraft_terrain::crafting::give_kit(&mut start, line);
        let stacks = |inventory: &Inventory| {
            let mut stacks: Vec<(String, u8, String)> = inventory
                .slots
                .iter()
                .flatten()
                .map(|stack| {
                    let components = stack
                        .components
                        .as_ref()
                        .map_or_else(String::new, ToString::to_string);
                    (stack.id.clone(), stack.count, components)
                })
                .collect();
            stacks.sort();
            stacks
        };
        let given = stacks(&parts.player(HOST).inventory);
        assert_eq!(given, stacks(&start));
        let potion = given
            .iter()
            .find(|(id, _, _)| id == "minecraft:potion")
            .unwrap();
        assert_eq!(potion.1, 2);
        assert!(
            potion
                .2
                .contains("\"minecraft:potion_contents\":{\"potion\":\"minecraft:healing\"}")
        );
        let arrows = given
            .iter()
            .find(|(id, _, _)| id == "minecraft:tipped_arrow")
            .unwrap();
        assert!(arrows.2.contains("minecraft:poison"), "{arrows:?}");
        let splash = given
            .iter()
            .find(|(id, _, _)| id == "minecraft:splash_potion")
            .unwrap();
        assert_eq!(splash.2, "", "an empty @ adds nothing");
        assert!(
            given
                .iter()
                .any(|(id, _, _)| id == "minecraft:leather_chestplate")
        );
    }

    #[test]
    fn locate_answers_once_the_level_has_searched() {
        let mut parts = Parts::new();
        let (lines, _) = parts.send(HOST, &["/locate structure minecraft:village_plains"]);
        assert!(lines.is_empty(), "the search waits for its level to open");
        assert_eq!(parts.waiting_locates.len(), 1);
        let (dimension, source, waiting) = &parts.waiting_locates[0];
        assert_eq!((*dimension, *source), (Dimension::Overworld, HOST));
        assert_eq!(waiting.printable, "minecraft:village_plains");
        let request = |kind: LocateKind, printable: &str, tag: bool| LocateRequest {
            kind,
            printable: printable.to_owned(),
            tag,
            targets: Vec::new(),
            dimension: minecraft_commands::OVERWORLD.to_owned(),
            origin: [0, 64, 0],
        };
        let job = |request: LocateRequest, search: Search| -> Box<dyn Any + Send> {
            Box::new(CommandJob::Located {
                source: HOST,
                request,
                search,
            })
        };
        let answers = answer_jobs(
            &parts.server,
            &parts.rules,
            HOST,
            vec![
                job(
                    request(LocateKind::Structure, "#minecraft:village", true),
                    Search::Found([272, 0, 944], "minecraft:village_plains".to_owned()),
                ),
                job(
                    request(LocateKind::Biome, "minecraft:desert", false),
                    Search::Found([608, 64, -1632], "minecraft:desert".to_owned()),
                ),
                job(
                    request(LocateKind::Structure, "minecraft:fortress", false),
                    Search::NotFound,
                ),
                job(
                    request(LocateKind::Structure, "minecraft:stronghold", false),
                    Search::TimedOut,
                ),
                job(
                    request(LocateKind::Poi, "minecraft:home", false),
                    Search::NotFound,
                ),
            ],
        );
        assert_eq!(
            answers.iter().map(line).collect::<Vec<_>>(),
            vec![
                "The nearest #minecraft:village (minecraft:village_plains) is at [272, ~, 944] (982 blocks away)",
                "The nearest minecraft:desert is at [608, 64, -1632] (1741 blocks away)",
                "Could not find a structure of type \"minecraft:fortress\" nearby",
                "Could not find a structure of type \"minecraft:stronghold\" nearby",
                "Could not find a point of interest of type \"minecraft:home\" within a reasonable distance",
            ]
        );
    }

    #[test]
    fn command_data_reads_the_item_tags_of_the_game_data() {
        let Some(stream) = test_stream() else {
            return;
        };
        let data = command_data(&stream);
        let logs = &data.item_tags["minecraft:logs"];
        for log in ["minecraft:oak_log", "minecraft:crimson_stem"] {
            assert!(logs.iter().any(|id| id == log), "{log} in #logs");
        }
        assert!(!logs.iter().any(|id| id.starts_with('#')));
        assert_eq!(data.blocks["minecraft:stone"], Vec::new());
        let stairs = &data.blocks["minecraft:oak_stairs"];
        let half = stairs.iter().find(|(name, _)| name == "half").unwrap();
        assert_eq!(half.1, vec!["top".to_owned(), "bottom".to_owned()]);
        assert!(data.block_tags["minecraft:logs"].contains(&"minecraft:oak_log".to_owned()));
        let has = |list: &[String], id: &str| list.iter().any(|known| known == id);
        assert!(has(&data.structures, "minecraft:village_plains"));
        assert!(has(
            &data.structure_tags["minecraft:village"],
            "minecraft:village_desert"
        ));
        assert!(has(&data.biomes, "minecraft:cherry_grove"));
        assert!(has(&data.biome_tags["minecraft:is_ocean"], "minecraft:deep_ocean"));
        assert!(has(&data.poi_types, "minecraft:home"));
        assert!(has(
            &data.poi_tags["minecraft:acquirable_job_site"],
            "minecraft:librarian"
        ));
    }

    #[test]
    fn commands_act_on_the_player_they_name_and_not_the_host() {
        let mut parts = Parts::new();
        parts.join(FRIEND, "Alex", [100.5, 64.0, 100.5]);
        parts.player(FRIEND).survival.status.health = 10.0;
        let (lines, teleports) = parts.send(
            HOST,
            &[
                "/gamemode creative Alex",
                "/effect give Alex regeneration 10",
                "/tp Alex ~ ~5 ~",
                "/kill Alex",
            ],
        );
        assert_eq!(
            lines,
            vec![
                "Set Alex's game mode to Creative Mode",
                "Applied effect Regeneration to Alex",
                "Teleported Alex to 4.500000, 75.000000, -2.500000",
                "Killed Alex",
            ]
        );
        assert_eq!(teleports, vec![(FRIEND, [4.5, 75.0, -2.5])]);
        assert_eq!(parts.player(FRIEND).mode.mode, GameMode::Creative);
        assert_eq!(parts.player(HOST).mode.mode, GameMode::Survival);
        assert!(
            parts
                .player(FRIEND)
                .survival
                .status
                .effect(EffectKind::Regeneration)
                .is_some()
        );
        assert!(
            parts
                .player(HOST)
                .survival
                .status
                .effect(EffectKind::Regeneration)
                .is_none()
        );
        assert!(parts.player(HOST).survival.status.health > 0.0);
        let mut host = parts.host();
        assert_eq!(host.game_mode(PlayerId(7)), None);
        assert!(!host.kill(PlayerId(7)));
        assert!(!host.teleport(PlayerId(7), [0.0; 3]));
        assert_eq!(host.mode_changed, Vec::<PlayerId>::new());
        assert!(host.set_game_mode(FRIEND, GameMode::Adventure));
        assert_eq!(host.mode_changed, vec![FRIEND]);
    }

    #[test]
    fn leftover_given_items_drop_at_the_feet() {
        let mut inventory = Inventory::default();
        let stone = ItemStack::new("minecraft:stone", 1);
        let (dropped, added) = give_stacks(&mut inventory, 0, &stone, 200);
        assert_eq!((dropped.len(), added), (0, 4));
        let counts: Vec<u8> = inventory.slots[..4]
            .iter()
            .map(|slot| slot.as_ref().map_or(0, |s| s.count))
            .collect();
        assert_eq!(counts, vec![64, 64, 64, 8]);
        for slot in 0..36 {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:dirt", 64));
        }
        inventory.slots[5] = Some(ItemStack::new("minecraft:stone", 60));
        let (dropped, added) = give_stacks(&mut inventory, 5, &stone, 70);
        assert_eq!(added, 0);
        let left: Vec<(String, u8)> = dropped.iter().map(|s| (s.id.clone(), s.count)).collect();
        assert_eq!(
            left,
            vec![
                ("minecraft:stone".to_owned(), 60),
                ("minecraft:stone".to_owned(), 6)
            ]
        );
        assert_eq!(inventory.slots[5].as_ref().map(|s| s.count), Some(64));
        let sword = ItemStack {
            max: 1,
            ..ItemStack::new("minecraft:diamond_sword", 1)
        };
        let (dropped, _) = give_stacks(&mut Inventory::default(), 0, &sword, 3);
        assert!(dropped.is_empty());
    }

    #[test]
    fn host_effects_reach_survival_and_teleports_wait_for_the_world() {
        let mut parts = Parts::new();
        parts.server.registries_mut().effects = [
            "minecraft:regeneration",
            "minecraft:speed",
            "minecraft:instant_health",
            "minecraft:saturation",
        ]
        .map(String::from)
        .to_vec();
        parts.player(HOST).survival.status.health = 10.0;
        let (delivered, teleports) = parts.send(
            HOST,
            &[
                "/effect give @s regeneration 10 1",
                "/effect give @s speed",
                "/effect give @s instant_health",
                "/tp ~ ~10 ~",
                "/tp 2000 120 2000",
                "/effect give @s minecraft:regeneration infinite",
            ],
        );
        assert_eq!(
            teleports,
            vec![(HOST, [4.5, 80.0, -2.5]), (HOST, [2000.5, 120.0, 2000.5])]
        );
        assert_eq!(delivered.len(), 6);
        let survival = &mut parts.player(HOST).survival;
        let regeneration = survival.status.effect(EffectKind::Regeneration).unwrap();
        assert_eq!(regeneration.amplifier, 1);
        assert_eq!(regeneration.duration, 200);
        assert_eq!(survival.status.health, 14.0);
        parts.presence[0].1.alive = false;
        let (_, teleports) = parts.send(
            HOST,
            &["/effect give @s regeneration infinite 1", "/tp 0 64 0"],
        );
        assert!(teleports.is_empty());
        let survival = &mut parts.player(HOST).survival;
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Regeneration)
                .unwrap()
                .duration,
            u32::MAX
        );
        survival.status.tick_effects();
        assert_eq!(
            survival
                .status
                .effect(EffectKind::Regeneration)
                .unwrap()
                .duration,
            u32::MAX - 1
        );
        let mut host = parts.host();
        assert!(host.effect_clear(HOST, Some("minecraft:regeneration")));
        assert!(!host.effect_clear(HOST, None));
        assert!(!host.effect_give(PlayerId(1), "minecraft:regeneration", Some(20), 0));
    }
}
