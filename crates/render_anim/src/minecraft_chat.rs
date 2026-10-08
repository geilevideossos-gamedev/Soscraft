use std::collections::HashMap;

use frame::{McChat, McChatLine};
use minecraft_commands::{Clientbound, CommandServer, PlayerId, Serverbound};

pub(crate) const HOST: PlayerId = PlayerId(0);
const HOST_LEVEL: u8 = 4;

pub(crate) fn pump(chat: &mut McChat, server: &mut CommandServer) -> Vec<Clientbound> {
    let mut delivered = Vec::new();
    for line in std::mem::take(&mut chat.sent) {
        let Some(message) = Serverbound::from_input(&line) else {
            continue;
        };
        delivered.extend(
            server
                .handle(HOST, message)
                .into_iter()
                .filter(|out| out.to.reaches(HOST, HOST_LEVEL))
                .map(|out| out.message),
        );
    }
    delivered
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
        ]
        .into_iter()
        .map(|(k, v)| (k.to_owned(), v.to_owned()))
        .collect()
    }

    #[test]
    fn sent_lines_come_back_as_resolved_chat_lines() {
        let mut server = CommandServer::default();
        server.join(HOST, "Steve");
        let mut chat = McChat::default();
        chat.sent.push("hello there".into());
        chat.sent.push("/frob".into());
        chat.sent.push("   ".into());
        let delivered = pump(&mut chat, &mut server);
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
}
