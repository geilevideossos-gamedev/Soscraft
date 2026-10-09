use std::collections::HashMap;

use frame::{McChat, McChatLine};
use minecraft_commands::{Clientbound, Clock, CommandRegistries, CommandServer, CommandWorld, ItemArg, PlayerId, Pose, Serverbound, Text, Weather};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_mode::{GameMode, PlayerMode};
use minecraft_terrain::game_rules::{GameRules, Rule, Value};
use minecraftoss_player::Difficulty;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::{Armor, EffectKind};

pub(crate) const HOST: PlayerId = PlayerId(0);

pub(crate) struct Host<'a> {
    pub(crate) seed: i64,
    pub(crate) rules: &'a mut GameRules,
    pub(crate) difficulty: &'a mut Difficulty,
    pub(crate) day: &'a mut DayCycle,
    pub(crate) save: Option<&'a mut minecraft_survival::save::WorldSave>,
    pub(crate) survival: &'a mut minecraft_survival::survival::Survival,
    pub(crate) entities: Option<&'a mut minecraft_survival::entities::Entities>,
    pub(crate) player_mode: &'a mut PlayerMode,
    pub(crate) rules_changed: bool,
    pub(crate) mode_changed: bool,
    pub(crate) pose: Option<Pose>,
    pub(crate) alive: bool,
    pub(crate) teleports: Vec<[f64; 3]>,
}

pub(crate) fn registries(packs: &minecraft_terrain::pack::PackStack) -> CommandRegistries {
    let mut items: Vec<String> = packs
        .list("minecraft", "items/")
        .unwrap_or_else(|_| Vec::new())
        .iter()
        .filter_map(|path| path.strip_prefix("assets/minecraft/items/")?.strip_suffix(".json"))
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
        entities: minecraft_terrain::server_mobs::SIMULATED_KINDS.iter().map(|kind| (*kind).to_owned()).collect(),
        effects,
        enchantments: Vec::new(),
    }
}

pub(crate) fn effect_kind(effect: &str) -> Option<EffectKind> {
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

pub(crate) fn give_stacks(inventory: &mut Inventory, selected: usize, stack: &ItemStack, count: u32) -> (Vec<ItemStack>, u32) {
    let max = u32::from(stack.max.max(1));
    let mut remaining = count;
    let mut dropped = Vec::new();
    let mut added = 0;
    while remaining > 0 {
        let size = remaining.min(max);
        remaining -= size;
        match inventory.add_item(ItemStack { count: size as u8, ..stack.clone() }, selected) {
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

pub(crate) fn summon_answers(server: &CommandServer, rules: &GameRules, answers: Vec<Result<String, String>>) -> Vec<Clientbound> {
    let level = server.level(HOST);
    answers
        .into_iter()
        .flat_map(|answer| match answer {
            Ok(kind) => server.feedback(HOST, Text::translate("commands.summon.success", vec![Text::entity(kind)]), true, rules),
            Err(error) => {
                let key = if error.contains("peaceful") {
                    "commands.summon.failed.peaceful"
                } else if error.contains("Invalid position") {
                    "commands.summon.invalidPosition"
                } else {
                    "commands.summon.failed"
                };
                vec![minecraft_commands::error_line(HOST, Text::translate(key, Vec::new()))]
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
            game_time: self.save.as_ref().map_or(total, |save| save.game_time(Some(self.day.ticks))),
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
        if let Some(entities) = self.entities.as_mut() {
            entities.set_weather(kind, duration);
        }
    }

    fn kill(&mut self, player: PlayerId) -> bool {
        if player != HOST {
            return false;
        }
        self.survival.kill(minecraft_survival::death::DamageCause::Kill);
        true
    }

    fn game_mode(&self, player: PlayerId) -> Option<GameMode> {
        (player == HOST).then_some(self.player_mode.mode)
    }

    fn set_game_mode(&mut self, player: PlayerId, mode: GameMode) -> bool {
        let changed = player == HOST && self.player_mode.set(mode);
        self.mode_changed |= changed;
        changed
    }

    fn pose(&self, player: PlayerId) -> Option<Pose> {
        if player == HOST { self.pose } else { None }
    }

    fn max_stack(&self, item: &str) -> u32 {
        self.entities.as_ref().map_or(64, |entities| u32::from(entities.inventory.recipes.max_stack(item)))
    }

    fn give(&mut self, player: PlayerId, item: &ItemArg, count: u32) -> bool {
        let (true, Some(entities)) = (player == HOST, self.entities.as_mut()) else {
            return false;
        };
        let stack = minecraft_terrain::crafting::kit_stack(&entities.inventory.recipes, &item.id, 1, &item.enchantments);
        let selected = entities.selected;
        let (dropped, added) = give_stacks(&mut entities.inventory, selected, &stack, count);
        if let Some(feet) = self.pose.map(|pose| pose.feet) {
            for stack in &dropped {
                entities.drop_given(stack, feet);
            }
            for _ in 0..added {
                let pitch = ((entities.random_float() - entities.random_float()) * 0.7 + 1.0) * 2.0;
                entities.sounds.push(("minecraft:entity.item.pickup".to_owned(), glam::DVec3::from_array(feet), 0.2, pitch));
            }
        }
        true
    }

    fn teleport(&mut self, player: PlayerId, position: [f64; 3]) -> bool {
        if player != HOST || !self.alive {
            return false;
        }
        self.teleports.push(position);
        if let Some(pose) = self.pose.as_mut() {
            pose.feet = position;
        }
        true
    }

    fn effect_give(&mut self, player: PlayerId, effect: &str, ticks: Option<u32>, amplifier: u8) -> bool {
        if player != HOST || self.survival.status.health <= 0.0 {
            return false;
        }
        let times = ticks.unwrap_or(1).clamp(1, 40);
        match effect {
            "minecraft:instant_health" => {
                let heal = 4_i32.wrapping_shl(u32::from(amplifier)).max(0) as f32;
                self.survival.status.heal(heal * times as f32);
                return true;
            }
            "minecraft:instant_damage" => {
                let damage = 6_i32.wrapping_shl(u32::from(amplifier)).max(0) as f32;
                self.survival.hurt(damage, Armor::default(), None, minecraft_survival::death::DamageCause::Magic);
                return true;
            }
            "minecraft:saturation" => {
                let food = u32::from(amplifier) + 1;
                for _ in 0..times {
                    self.survival.status.food.eat(food.min(20) as u8, 2.0 * food as f32);
                }
                return true;
            }
            _ => {}
        }
        let Some(kind) = effect_kind(effect) else {
            return false;
        };
        let status = &mut self.survival.status;
        let before = status.effect(kind);
        status.add_effect(kind, ticks.unwrap_or(u32::MAX), amplifier);
        status.effect(kind) != before
    }

    fn effect_clear(&mut self, player: PlayerId, effect: Option<&str>) -> bool {
        if player != HOST {
            return false;
        }
        let status = &mut self.survival.status;
        match effect {
            Some(effect) => match effect_kind(effect).filter(|kind| status.effect(*kind).is_some()) {
                Some(kind) => {
                    status.remove_effect(kind);
                    true
                }
                None => false,
            },
            None => {
                let kinds = [EffectKind::Hunger, EffectKind::Poison, EffectKind::Regeneration, EffectKind::Absorption, EffectKind::Resistance, EffectKind::FireResistance, EffectKind::Nausea, EffectKind::Slowness, EffectKind::Weakness];
                let any = kinds.iter().any(|kind| status.effect(*kind).is_some());
                self.survival.clear_effects();
                any
            }
        }
    }

    fn summon(&mut self, _source: PlayerId, kind: &str, position: [f64; 3], nbt: Option<&str>) -> Result<(), Text> {
        let tag = nbt.map(minecraftoss_core::snbt::parse_compound).transpose().map_err(|error| nbt_error(&error))?;
        let Some(entities) = self.entities.as_mut() else {
            return Err(Text::translate("commands.summon.failed", Vec::new()));
        };
        entities.summon(kind.to_owned(), position, tag);
        Ok(())
    }
}

pub(crate) fn pump(chat: &mut McChat, server: &mut CommandServer, world: &mut dyn CommandWorld) -> Vec<Clientbound> {
    let mut delivered = Vec::new();
    for line in std::mem::take(&mut chat.sent) {
        let Some(message) = Serverbound::from_input(&line) else {
            continue;
        };
        let level = server.level(HOST);
        delivered.extend(
            server
                .handle(HOST, message, world)
                .into_iter()
                .filter(|out| out.to.reaches(HOST, level))
                .map(|out| out.message),
        );
    }
    delivered
}

pub(crate) fn death_message(line: String) -> Clientbound {
    Clientbound::System {
        text: Text::literal(line),
        overlay: false,
    }
}

pub(crate) fn show(
    chat: &mut McChat,
    messages: &[Clientbound],
    language: &HashMap<String, String>,
) {
    let lang = |key: &str| language.get(key).cloned();
    for message in messages {
        let Some(text) = message.chat_text() else {
            continue;
        };
        let resolved = text.resolve(&lang);
        diag::info!(World, "[CHAT] {}", resolved.text);
        chat.received.push(McChatLine {
            text: resolved.text,
            color: resolved.color,
            spans: resolved.spans,
            italic: resolved.italic,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn language() -> HashMap<String, String> {
        [
            ("chat.type.text", "<%s> %s"),
            (
                "command.unknown.command",
                "Unknown or incomplete command. See below for error",
            ),
            ("command.context.here", "<--[HERE]"),
            ("commands.kill.success.single", "Killed %s"),
            ("commands.time.set.absolute", "Set %s to %s tick(s)"),
            ("commands.gamerule.set", "Game rule %s is now set to %s"),
            ("commands.gamemode.success.self", "Set own game mode to %s"),
            ("gameMode.creative", "Creative Mode"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    struct Parts {
        rules: GameRules,
        difficulty: Difficulty,
        day: DayCycle,
        survival: minecraft_survival::survival::Survival,
        player_mode: PlayerMode,
    }

    impl Parts {
        fn new() -> Self {
            let mut survival = minecraft_survival::survival::Survival::default();
            survival.observe(100, 100);
            Self {
                rules: GameRules::default(),
                difficulty: Difficulty::Normal,
                day: DayCycle::default(),
                survival,
                player_mode: PlayerMode::default(),
            }
        }

        fn host(&mut self) -> Host<'_> {
            Host {
                seed: 42,
                rules: &mut self.rules,
                difficulty: &mut self.difficulty,
                day: &mut self.day,
                save: None,
                survival: &mut self.survival,
                entities: None,
                player_mode: &mut self.player_mode,
                rules_changed: false,
                mode_changed: false,
                pose: Some(Pose { feet: [4.5, 70.0, -2.5], yaw: 0.0, pitch: 0.0 }),
                alive: true,
                teleports: Vec::new(),
            }
        }
    }

    #[test]
    fn sent_lines_come_back_as_resolved_chat_lines() {
        let mut server = CommandServer::default();
        server.join(HOST, "Steve");
        let mut parts = Parts::new();
        let mut chat = McChat::default();
        chat.sent.push("hello there".into());
        chat.sent.push("/frob".into());
        chat.sent.push("   ".into());
        let delivered = pump(&mut chat, &mut server, &mut parts.host());
        assert!(chat.sent.is_empty());
        assert_eq!(delivered.len(), 3);
        show(&mut chat, &delivered, &language());
        let texts: Vec<&str> = chat
            .received
            .iter()
            .map(|line| line.text.as_str())
            .collect();
        assert_eq!(
            texts,
            vec![
                "<Steve> hello there",
                "Unknown or incomplete command. See below for error",
                "frob<--[HERE]",
            ]
        );
        assert_eq!(chat.received[0].color, minecraft_commands::text::WHITE);
        assert_eq!(chat.received[1].color, minecraft_commands::text::RED);
        assert_eq!(chat.received[2].color, minecraft_commands::text::GRAY);
        assert_eq!(
            chat.received[2].spans,
            vec![(0..13, minecraft_commands::text::RED)]
        );
    }

    #[test]
    fn host_commands_reach_survival_the_clock_and_the_rules() {
        let mut server = CommandServer::new(HOST);
        server.join(HOST, "Steve");
        let mut parts = Parts::new();
        let mut chat = McChat::default();
        chat.sent.push("/time set 2000".into());
        chat.sent.push("/gamerule keep_inventory true".into());
        chat.sent.push("/kill".into());
        let mut host = parts.host();
        let delivered = pump(&mut chat, &mut server, &mut host);
        assert!(host.rules_changed);
        show(&mut chat, &delivered, &language());
        let texts: Vec<&str> = chat.received.iter().map(|line| line.text.as_str()).collect();
        assert_eq!(
            texts,
            vec![
                "Set minecraft:overworld to 2000 tick(s)",
                "Game rule keep_inventory is now set to true",
                "Killed Steve",
            ]
        );
        assert_eq!(parts.day.ticks, 2000.0);
        assert!(parts.rules.keep_inventory());
        assert!(parts.survival.killed());
        assert_eq!(parts.survival.status.health, 0.0);
    }

    #[test]
    fn gamemode_switches_the_host_player_mode() {
        let mut server = CommandServer::new(HOST);
        server.join(HOST, "Steve");
        let mut parts = Parts::new();
        let mut chat = McChat::default();
        chat.sent.push("/gamemode creative".into());
        let mut host = parts.host();
        let delivered = pump(&mut chat, &mut server, &mut host);
        assert!(host.mode_changed);
        assert_eq!(host.game_mode(HOST), Some(GameMode::Creative));
        assert_eq!(host.game_mode(PlayerId(1)), None);
        assert!(!host.set_game_mode(PlayerId(1), GameMode::Spectator));
        show(&mut chat, &delivered, &language());
        assert_eq!(chat.received[0].text, "Set own game mode to Creative Mode");
        assert_eq!(parts.player_mode.mode, GameMode::Creative);
        assert_eq!(parts.player_mode.previous, Some(GameMode::Survival));
        assert!(parts.player_mode.abilities.invulnerable);
        chat.sent.push("/gamemode creative".into());
        let mut host = parts.host();
        assert!(pump(&mut chat, &mut server, &mut host).is_empty());
        assert!(!host.mode_changed);
    }

    #[test]
    fn death_messages_are_plain_system_lines() {
        let mut chat = McChat::default();
        show(&mut chat, &[death_message("Steve was killed".into())], &HashMap::new());
        assert_eq!(chat.received[0].text, "Steve was killed");
        assert_eq!(chat.received[0].color, minecraft_commands::text::WHITE);
    }

    #[test]
    fn leftover_given_items_drop_at_the_feet() {
        let mut inventory = Inventory::default();
        let stone = ItemStack::new("minecraft:stone", 1);
        let (dropped, added) = give_stacks(&mut inventory, 0, &stone, 200);
        assert_eq!((dropped.len(), added), (0, 4));
        let counts: Vec<u8> = inventory.slots[..4].iter().map(|slot| slot.as_ref().map_or(0, |s| s.count)).collect();
        assert_eq!(counts, vec![64, 64, 64, 8]);
        for slot in 0..36 {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:dirt", 64));
        }
        inventory.slots[5] = Some(ItemStack::new("minecraft:stone", 60));
        let (dropped, added) = give_stacks(&mut inventory, 5, &stone, 70);
        assert_eq!(added, 0);
        let left: Vec<(String, u8)> = dropped.iter().map(|s| (s.id.clone(), s.count)).collect();
        assert_eq!(left, vec![("minecraft:stone".to_owned(), 60), ("minecraft:stone".to_owned(), 6)]);
        assert_eq!(inventory.slots[5].as_ref().map(|s| s.count), Some(64));
        let sword = ItemStack { max: 1, ..ItemStack::new("minecraft:diamond_sword", 1) };
        let (dropped, _) = give_stacks(&mut Inventory::default(), 0, &sword, 3);
        assert!(dropped.is_empty());
    }

    #[test]
    fn host_effects_reach_survival_and_teleports_wait_for_the_world() {
        let mut server = CommandServer::new(HOST);
        server.join(HOST, "Steve");
        server.registries_mut().effects = ["minecraft:regeneration", "minecraft:speed", "minecraft:instant_health", "minecraft:saturation"].map(String::from).to_vec();
        let mut parts = Parts::new();
        parts.survival.status.health = 10.0;
        let mut chat = McChat::default();
        for line in ["/effect give @s regeneration 10 1", "/effect give @s speed", "/effect give @s instant_health", "/tp ~ ~10 ~", "/tp 2000 120 2000", "/effect give @s minecraft:regeneration infinite"] {
            chat.sent.push(line.into());
        }
        let mut host = parts.host();
        let delivered = pump(&mut chat, &mut server, &mut host);
        assert_eq!(host.teleports, vec![[4.5, 80.0, -2.5], [2000.5, 120.0, 2000.5]]);
        assert_eq!(delivered.len(), 6);
        let regeneration = parts.survival.status.effect(EffectKind::Regeneration).unwrap();
        assert_eq!(regeneration.amplifier, 1);
        assert_eq!(regeneration.duration, 200);
        assert_eq!(parts.survival.status.health, 14.0);
        let mut host = parts.host();
        host.alive = false;
        chat.sent.push("/effect give @s regeneration infinite 1".into());
        chat.sent.push("/tp 0 64 0".into());
        pump(&mut chat, &mut server, &mut host);
        assert!(host.teleports.is_empty());
        assert_eq!(parts.survival.status.effect(EffectKind::Regeneration).unwrap().duration, u32::MAX);
        parts.survival.status.tick_effects();
        assert_eq!(parts.survival.status.effect(EffectKind::Regeneration).unwrap().duration, u32::MAX - 1);
        let mut host = parts.host();
        assert!(host.effect_clear(HOST, Some("minecraft:regeneration")));
        assert!(!host.effect_clear(HOST, None));
        assert!(!host.effect_give(PlayerId(1), "minecraft:regeneration", Some(20), 0));
    }

    #[test]
    fn summon_answers_follow_the_feedback_rule() {
        let mut server = CommandServer::new(HOST);
        server.join(HOST, "Steve");
        let mut rules = GameRules::default();
        let answers = summon_answers(&server, &rules, vec![Ok("minecraft:cow".into()), Err("You can not summon a monster in peaceful mode".into()), Err("Unable to summon minecraft:fox".into())]);
        let language: HashMap<String, String> = [("commands.summon.success", "Summoned new %s"), ("entity.minecraft.cow", "Cow"), ("commands.summon.failed.peaceful", "Monsters cannot be summoned in Peaceful difficulty"), ("commands.summon.failed", "Unable to summon entity")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
        let mut chat = McChat::default();
        show(&mut chat, &answers, &language);
        let texts: Vec<(&str, u32)> = chat.received.iter().map(|line| (line.text.as_str(), line.color)).collect();
        assert_eq!(
            texts,
            vec![
                ("Summoned new Cow", minecraft_commands::text::WHITE),
                ("Monsters cannot be summoned in Peaceful difficulty", minecraft_commands::text::RED),
                ("Unable to summon entity", minecraft_commands::text::RED),
            ]
        );
        rules.set("send_command_feedback", "false").unwrap();
        assert_eq!(summon_answers(&server, &rules, vec![Ok("minecraft:cow".into()), Err("x".into())]).len(), 1);
    }
}
