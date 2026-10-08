use crate::protocol::{
    Clientbound, Outgoing, PlayerId, Recipient, Serverbound, allowed_in_chat, normalize_chat,
};
use crate::text::{GRAY, RED, Text};

#[derive(Clone, Debug, Default)]
pub struct CommandServer {
    players: Vec<(PlayerId, String)>,
}

impl CommandServer {
    pub fn join(&mut self, id: PlayerId, name: impl Into<String>) {
        let name = name.into();
        match self.players.iter_mut().find(|(player, _)| *player == id) {
            Some((_, known)) => *known = name,
            None => self.players.push((id, name)),
        }
    }

    pub fn leave(&mut self, id: PlayerId) {
        self.players.retain(|(player, _)| *player != id);
    }

    pub fn name(&self, id: PlayerId) -> Option<&str> {
        self.players
            .iter()
            .find(|(player, _)| *player == id)
            .map(|(_, name)| name.as_str())
    }

    pub fn handle(&mut self, from: PlayerId, message: Serverbound) -> Vec<Outgoing> {
        let Some(name) = self.name(from).map(str::to_owned) else {
            return Vec::new();
        };
        match message {
            Serverbound::Chat(line) => {
                let filtered: String = line.chars().filter(|c| allowed_in_chat(*c)).collect();
                let message = normalize_chat(&filtered);
                if message.is_empty() {
                    return Vec::new();
                }
                vec![Outgoing {
                    to: Recipient::All,
                    message: Clientbound::Player {
                        sender: from,
                        name,
                        message,
                    },
                }]
            }
            Serverbound::Command(command) => failure(
                from,
                Text::translate("command.unknown.command", Vec::new()),
                &command,
                0,
            ),
        }
    }
}

pub fn failure(to: PlayerId, error: Text, input: &str, cursor: usize) -> Vec<Outgoing> {
    let system = |text: Text| Outgoing {
        to: Recipient::Player(to),
        message: Clientbound::System {
            text,
            overlay: false,
        },
    };
    let mut cursor = cursor.min(input.len());
    while !input.is_char_boundary(cursor) {
        cursor -= 1;
    }
    let before: String = {
        let head = &input[..cursor];
        let skip = head.chars().count().saturating_sub(10);
        head.chars().skip(skip).collect()
    };
    let mut context = Text::literal("").color(GRAY);
    if input[..cursor].chars().count() > 10 {
        context = context.append(Text::literal("..."));
    }
    context = context.append(Text::literal(before));
    if cursor < input.len() {
        context = context.append(Text::literal(&input[cursor..]).color(RED).underlined());
    }
    context = context.append(
        Text::translate("command.context.here", Vec::new())
            .color(RED)
            .italic(),
    );
    vec![
        system(Text::literal("").color(RED).append(error)),
        system(context),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOST: PlayerId = PlayerId(0);

    fn lang(key: &str) -> Option<String> {
        match key {
            "chat.type.text" => Some("<%s> %s".into()),
            "command.unknown.command" => {
                Some("Unknown or incomplete command. See below for error".into())
            }
            "command.context.here" => Some("<--[HERE]".into()),
            _ => None,
        }
    }

    fn server() -> CommandServer {
        let mut server = CommandServer::default();
        server.join(HOST, "Steve");
        server
    }

    #[test]
    fn chat_broadcasts_the_sender_name() {
        let mut server = server();
        let out = server.handle(HOST, Serverbound::Chat("  hello\u{a7}  there ".into()));
        assert_eq!(
            out,
            vec![Outgoing {
                to: Recipient::All,
                message: Clientbound::Player {
                    sender: HOST,
                    name: "Steve".into(),
                    message: "hello there".into(),
                },
            }]
        );
        let text = out[0].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(text.text, "<Steve> hello there");
        assert!(
            server
                .handle(HOST, Serverbound::Chat("   ".into()))
                .is_empty()
        );
        assert!(
            server
                .handle(PlayerId(7), Serverbound::Chat("who".into()))
                .is_empty()
        );
        server.join(HOST, "Alex");
        assert_eq!(server.name(HOST), Some("Alex"));
        server.leave(HOST);
        assert_eq!(server.name(HOST), None);
    }

    #[test]
    fn a_command_without_dispatcher_is_unknown_and_red() {
        let mut server = server();
        let out = server.handle(HOST, Serverbound::Command("frob".into()));
        assert_eq!(out.len(), 2);
        assert!(out.iter().all(|o| o.to == Recipient::Player(HOST)));
        let lines: Vec<_> = out
            .iter()
            .map(|o| o.message.chat_text().unwrap().resolve(&lang))
            .collect();
        assert_eq!(
            lines[0].text,
            "Unknown or incomplete command. See below for error"
        );
        assert_eq!(lines[0].color, RED);
        assert_eq!(lines[1].text, "frob<--[HERE]");
        assert_eq!(lines[1].color, GRAY);
        assert_eq!(lines[1].spans, vec![(0..13, RED)]);
    }

    #[test]
    fn the_error_context_shows_ten_chars_before_the_cursor() {
        let out = failure(
            HOST,
            Text::literal("bad"),
            "gamerule keep_inventory maybe",
            24,
        );
        let context = out[1].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(context.text, "...inventory maybe<--[HERE]");
        assert_eq!(context.spans, vec![(13..27, RED)]);
        let at_end = failure(HOST, Text::literal("bad"), "seed", 4);
        let context = at_end[1].message.chat_text().unwrap().resolve(&lang);
        assert_eq!(context.text, "seed<--[HERE]");
        assert_eq!(context.spans, vec![(4..13, RED)]);
    }
}
