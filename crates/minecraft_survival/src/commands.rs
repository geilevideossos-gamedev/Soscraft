use std::any::Any;
use std::collections::{BTreeMap, HashMap};

use minecraft_commands::{
    Answer, BlockEdit, Clientbound, Clock, CommandData, CommandRegistries, CommandServer,
    CommandWorld, Enchanted, EnchantmentInfo, Experience, ItemArg, ItemFilter, Loaded,
    LocateKind, LocateRequest, Located, PlayerId, Pose, Serverbound, Text, Weather,
};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::locate::Search;
use minecraft_terrain::server::ServerSim;
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
    pub save: Option<&'a mut crate::save::WorldSave>,
    pub data: &'a crate::data::GameData,
    pub levels: &'a mut BTreeMap<Dimension, Level>,
    pub players: &'a mut BTreeMap<PlayerUuid, Player>,
    pub presence: Vec<(PlayerUuid, Presence)>,
    pub rules_changed: bool,
    pub mode_changed: Vec<PlayerId>,
    pub teleports: Vec<(PlayerId, [f64; 3])>,
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

    fn clock(&self) -> Clock {
        let total = self.day.ticks.floor() as i64;
        Clock {
            total,
            game_time: self
                .save
                .as_ref()
                .map_or(total, |save| save.game_time(Some(self.day.ticks))),
            paused: self.day.paused,
        }
    }

    fn set_time(&mut self, total: i64) {
        let ticks = total as f64;
        if let Some(save) = self.save.as_mut() {
            save.shift_day(ticks - self.day.ticks);
        }
        self.day.set(ticks);
    }

    fn set_time_paused(&mut self, paused: bool) {
        self.day.paused = paused;
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
        let Some((player, Some(entities))) = self.player_in_level(player) else {
            return false;
        };
        let stack = minecraft_terrain::crafting::kit_stack(
            &entities.recipes,
            &item.id,
            1,
            &item.enchantments,
        );
        let (dropped, added) =
            give_stacks(&mut player.inventory, player.selected, &stack, count);
        if let Some(feet) = feet {
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

    fn locate(&mut self, source: PlayerId, request: &LocateRequest) -> Answer<Option<Located>> {
        let Some(level) = loaded_dimension(self.levels, &request.dimension)
            .and_then(|dimension| self.levels.get_mut(&dimension))
        else {
            return Answer::Now(None);
        };
        let worldgen = level.terrain.stream.world_gen();
        let Some(entities) = level.entities.as_mut() else {
            return Answer::Now(None);
        };
        let request = request.clone();
        entities.run_job(Box::new(move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
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
        }));
        Answer::Later
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
        save: globals.save.as_mut(),
        data: &globals.data,
        levels: &mut *levels,
        players: &mut *players,
        presence,
        rules_changed: false,
        mode_changed: Vec::new(),
        teleports: Vec::new(),
    };
    let mut delivered = pump(id, sent, &mut globals.chat, &mut host);
    let rules_changed = host.rules_changed;
    let mode_changed = std::mem::take(&mut host.mode_changed);
    let teleports = std::mem::take(&mut host.teleports);
    drop(host);
    let mut finished = Vec::new();
    for entities in levels.values_mut().filter_map(|level| level.entities.as_mut()) {
        finished.append(&mut entities.command_jobs);
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
    if let Some(player) = players.get(from)
        && mode_changed.contains(&player.id)
    {
        crate::server::save_player_mode(globals, player.mode);
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
        pub(crate) data: crate::data::GameData,
        pub(crate) levels: BTreeMap<Dimension, Level>,
        pub(crate) players: BTreeMap<PlayerUuid, Player>,
        pub(crate) presence: Vec<(PlayerUuid, Presence)>,
        pub(crate) server: CommandServer,
    }

    impl Parts {
        pub(crate) fn new() -> Self {
            let mut parts = Self {
                rules: GameRules::default(),
                difficulty: Difficulty::Normal,
                day: DayCycle::default(),
                data: crate::data::GameData::default(),
                levels: BTreeMap::new(),
                players: BTreeMap::new(),
                presence: Vec::new(),
                server: CommandServer::new(HOST),
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
                save: None,
                data: &self.data,
                levels: &mut self.levels,
                players: &mut self.players,
                presence: self.presence.clone(),
                rules_changed: false,
                mode_changed: Vec::new(),
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
    fn locate_answers_once_the_level_has_searched() {
        let mut parts = Parts::new();
        let (lines, _) = parts.send(HOST, &["/locate structure minecraft:village_plains"]);
        assert_eq!(
            lines,
            vec!["Could not find a structure of type \"minecraft:village_plains\" nearby"]
        );
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
