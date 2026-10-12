use std::collections::HashMap;

use frame::MinecraftUi;
use frame::minecraft_ui::McOverlayMessage;
use minecraft_commands::Clientbound;
use minecraft_terrain::pack::PackStack;

use crate::minecraft_inventory::InventoryUi;

const TICKS_PER_SECOND: f64 = 20.0;

pub(crate) fn show_messages(
    ui: &mut MinecraftUi,
    inventory_ui: &InventoryUi,
    packs: &PackStack,
    messages: &[Clientbound],
) {
    if messages.is_empty() {
        return;
    }
    let loaded;
    let language = match inventory_ui.language() {
        Some(language) => language,
        None => {
            loaded =
                minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new());
            &loaded
        }
    };
    let lang = |key: &str| language.get(key).cloned();
    for message in messages {
        if let Clientbound::System {
            text,
            overlay: true,
        } = message
        {
            ui.overlay_message = Some(McOverlayMessage {
                text: text.resolve(&lang).text,
                ticks: McOverlayMessage::TICKS,
            });
        }
    }
}

pub(crate) fn show_overlay(ui: &mut MinecraftUi, language: &HashMap<String, String>, key: &str) {
    ui.overlay_message = Some(McOverlayMessage {
        text: language.get(key).cloned().unwrap_or_else(|| key.to_owned()),
        ticks: McOverlayMessage::TICKS,
    });
}

pub(crate) fn fade_overlay(ui: &mut MinecraftUi, seconds: f64) {
    if let Some(message) = ui.overlay_message.as_mut() {
        message.ticks -= (seconds * TICKS_PER_SECOND) as f32;
    }
    ui.overlay_message.take_if(|message| message.ticks <= 0.0);
}
