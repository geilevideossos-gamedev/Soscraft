use std::collections::HashMap;

use frame::{McChat, McChatLine};
use minecraft_commands::Clientbound;

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
    use minecraft_commands::{CommandServer, CommandWorld, PlayerId, Pose};
    use minecraft_survival::commands::{HOST, Host, death_message, pump, summon_answers};
    use minecraft_terrain::day_cycle::DayCycle;
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};
    use minecraft_terrain::game_rules::GameRules;
    use minecraftoss_player::Difficulty;

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
        inventory: minecraftoss_player::inventory::Inventory,
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
                inventory: minecraftoss_player::inventory::Inventory::default(),
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
                inventory: &mut self.inventory,
                selected: 0,
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
        let delivered = pump(&mut chat.sent, &mut server, &mut parts.host());
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
        let delivered = pump(&mut chat.sent, &mut server, &mut host);
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
        let delivered = pump(&mut chat.sent, &mut server, &mut host);
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
        assert!(pump(&mut chat.sent, &mut server, &mut host).is_empty());
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
