use std::collections::{BTreeMap, HashMap};

use minecraft_commands::{
    Clientbound, Clock, CommandData, CommandRegistries, CommandServer, CommandWorld, ItemArg,
    ItemFilter, PlayerId, Pose, Serverbound, Text, Weather,
};
use minecraft_net::{Dimension, PlayerUuid};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::terrain::TerrainStream;
use minecraft_terrain::game_mode::GameMode;
use minecraft_terrain::game_rules::{GameRules, Rule, Value};
use minecraftoss_player::Difficulty;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{Armor, EffectKind};

use crate::entities::Entities;
use crate::level::Level;
use crate::player::Player;
use crate::server::Globals;

pub const HOST: PlayerId = PlayerId(0);

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
}

pub fn command_data(stream: &TerrainStream) -> CommandData {
    let worldgen = stream.world_gen();
    CommandData {
        item_tags: minecraft_terrain::locate::tags(&worldgen, "item"),
        ..CommandData::default()
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
    let delivered = pump(id, sent, &mut globals.chat, &mut host);
    let rules_changed = host.rules_changed;
    let mode_changed = std::mem::take(&mut host.mode_changed);
    let teleports = std::mem::take(&mut host.teleports);
    drop(host);
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
