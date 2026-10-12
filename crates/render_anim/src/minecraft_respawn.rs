use std::collections::HashMap;

use frame::MinecraftUi;
use frame::minecraft_ui::McOverlayMessage;

const TICKS_PER_SECOND: f64 = 20.0;

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
