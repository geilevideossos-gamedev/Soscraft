use serde::{Deserialize, Serialize};

use crate::text::Text;

pub const MAX_CHAT_LENGTH: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlayerId(pub u32);

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Serverbound {
    Chat(String),
    Command(String),
}

impl Serverbound {
    pub fn from_input(line: &str) -> Option<Self> {
        let line = normalize_chat(line);
        if line.is_empty() {
            return None;
        }
        Some(match line.strip_prefix('/') {
            Some(command) => Self::Command(command.to_owned()),
            None => Self::Chat(line),
        })
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Clientbound {
    System {
        text: Text,
        overlay: bool,
    },
    Player {
        sender: PlayerId,
        name: String,
        message: String,
    },
    Commands {
        level: u8,
    },
}

impl Clientbound {
    pub fn chat_text(&self) -> Option<Text> {
        match self {
            Self::System {
                text,
                overlay: false,
            } => Some(text.clone()),
            Self::Player { name, message, .. } => Some(Text::translate(
                "chat.type.text",
                vec![Text::literal(name.clone()), Text::literal(message.clone())],
            )),
            Self::System { overlay: true, .. } | Self::Commands { .. } => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Recipient {
    Player(PlayerId),
    All,
    Ops(u8),
}

impl Recipient {
    pub fn reaches(self, player: PlayerId, level: u8) -> bool {
        match self {
            Self::Player(to) => to == player,
            Self::All => true,
            Self::Ops(at_least) => level >= at_least,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Outgoing {
    pub to: Recipient,
    pub message: Clientbound,
}

pub fn allowed_in_chat(c: char) -> bool {
    c != '\u{a7}' && !c.is_control()
}

pub fn normalize_chat(line: &str) -> String {
    line.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(MAX_CHAT_LENGTH)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_collapses_spaces_and_caps_256() {
        assert_eq!(
            normalize_chat("  hello   there \t you  "),
            "hello there you"
        );
        assert_eq!(normalize_chat("   "), "");
        let long = "é".repeat(300);
        let normalized = normalize_chat(&long);
        assert_eq!(normalized.chars().count(), MAX_CHAT_LENGTH);
        assert!(normalized.chars().all(|c| c == 'é'));
    }

    #[test]
    fn input_lines_become_chat_or_commands() {
        assert_eq!(
            Serverbound::from_input("  hi  you "),
            Some(Serverbound::Chat("hi you".into()))
        );
        assert_eq!(
            Serverbound::from_input("/time  set noon"),
            Some(Serverbound::Command("time set noon".into()))
        );
        assert_eq!(Serverbound::from_input("  "), None);
        assert!(!allowed_in_chat('\u{a7}'));
        assert!(!allowed_in_chat('\u{7f}'));
        assert!(allowed_in_chat('@'));
    }

    #[test]
    fn messages_round_trip_through_json() {
        let serverbound = [
            Serverbound::Chat("hello".into()),
            Serverbound::Command("seed".into()),
        ];
        for message in serverbound {
            let json = serde_json::to_string(&message).unwrap();
            assert_eq!(serde_json::from_str::<Serverbound>(&json).unwrap(), message);
        }
        let clientbound = [
            Clientbound::System {
                text: Text::translate("command.unknown.command", Vec::new())
                    .color(crate::text::RED)
                    .append(Text::item("minecraft:stone")),
                overlay: false,
            },
            Clientbound::Player {
                sender: PlayerId(3),
                name: "Steve".into(),
                message: "hi".into(),
            },
            Clientbound::Commands { level: 4 },
        ];
        for message in clientbound {
            let json = serde_json::to_string(&message).unwrap();
            assert_eq!(serde_json::from_str::<Clientbound>(&json).unwrap(), message);
        }
        let outgoing = Outgoing {
            to: Recipient::Ops(2),
            message: Clientbound::Commands { level: 0 },
        };
        let json = serde_json::to_string(&outgoing).unwrap();
        assert_eq!(serde_json::from_str::<Outgoing>(&json).unwrap(), outgoing);
    }

    #[test]
    fn recipients_reach_the_right_players() {
        let host = PlayerId(0);
        let friend = PlayerId(1);
        assert!(Recipient::Player(host).reaches(host, 0));
        assert!(!Recipient::Player(host).reaches(friend, 4));
        assert!(Recipient::All.reaches(friend, 0));
        assert!(Recipient::Ops(2).reaches(host, 4));
        assert!(!Recipient::Ops(2).reaches(friend, 1));
    }

    #[test]
    fn player_messages_show_as_vanilla_chat_lines() {
        let line = Clientbound::Player {
            sender: PlayerId(0),
            name: "Steve".into(),
            message: "hi".into(),
        };
        let lang = |key: &str| (key == "chat.type.text").then(|| "<%s> %s".to_owned());
        assert_eq!(line.chat_text().unwrap().resolve(&lang).text, "<Steve> hi");
        let overlay = Clientbound::System {
            text: Text::literal("x"),
            overlay: true,
        };
        assert_eq!(overlay.chat_text(), None);
        assert_eq!(Clientbound::Commands { level: 4 }.chat_text(), None);
    }
}
