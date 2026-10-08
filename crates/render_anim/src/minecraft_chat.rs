use std::collections::HashMap;

use frame::{McChat, McChatLine};
use minecraft_commands::{Clientbound, Clock, CommandServer, CommandWorld, PlayerId, Serverbound, Text, Weather};
use minecraft_terrain::day_cycle::DayCycle;
use minecraft_terrain::game_rules::{GameRules, Rule, Value};
use minecraftoss_player::Difficulty;

pub(crate) const HOST: PlayerId = PlayerId(0);

pub(crate) struct Host<'a> {
    pub(crate) seed: i64,
    pub(crate) rules: &'a mut GameRules,
    pub(crate) difficulty: &'a mut Difficulty,
    pub(crate) day: &'a mut DayCycle,
    pub(crate) save: Option<&'a mut crate::minecraft_save::WorldSave>,
    pub(crate) survival: &'a mut crate::minecraft_survival::Survival,
    pub(crate) entities: Option<&'a mut crate::minecraft_entities::Entities>,
    pub(crate) rules_changed: bool,
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
        self.survival.kill(crate::minecraft_death::DamageCause::Kill);
        true
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
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    struct Parts {
        rules: GameRules,
        difficulty: Difficulty,
        day: DayCycle,
        survival: crate::minecraft_survival::Survival,
    }

    impl Parts {
        fn new() -> Self {
            let mut survival = crate::minecraft_survival::Survival::default();
            survival.observe(100, 100);
            Self {
                rules: GameRules::default(),
                difficulty: Difficulty::Normal,
                day: DayCycle::default(),
                survival,
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
                rules_changed: false,
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
    fn death_messages_are_plain_system_lines() {
        let mut chat = McChat::default();
        show(&mut chat, &[death_message("Steve was killed".into())], &HashMap::new());
        assert_eq!(chat.received[0].text, "Steve was killed");
        assert_eq!(chat.received[0].color, minecraft_commands::text::WHITE);
    }
}
