use std::collections::HashMap;

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use minecraft_survival::actions::{Effect, Hands};
use minecraft_terrain::mesh::Atlas;
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::HandcraftedScene;

use crate::minecraft_inventory::InventoryUi;
use crate::minecraft_mining::Mining;
use crate::minecraft_sounds::Sounds;

pub(crate) fn host_hands(
    ui: &frame::MinecraftUi,
    hud_input: Option<&frame::HudInputView>,
    buttons: &ButtonInput<MouseButton>,
    pad: Option<&bevy::input::gamepad::Gamepad>,
) -> Hands {
    let menu_captured =
        ui.chat.open || hud_input.is_some_and(|input| input.console_open || input.script_menu_open);
    Hands {
        attack_held: buttons.pressed(MouseButton::Left)
            || pad_trigger(pad, GamepadButton::RightTrigger2, false),
        attack_pressed: buttons.just_pressed(MouseButton::Left)
            || pad_trigger(pad, GamepadButton::RightTrigger2, true),
        use_held: buttons.pressed(MouseButton::Right)
            || pad_trigger(pad, GamepadButton::LeftTrigger2, false),
        use_pressed: buttons.just_pressed(MouseButton::Right)
            || pad_trigger(pad, GamepadButton::LeftTrigger2, true),
        offhand_key: ui.offhand_key == Some(true),
        captured: ui.inventory_open || menu_captured,
    }
}

fn pad_trigger(
    pad: Option<&bevy::input::gamepad::Gamepad>,
    button: GamepadButton,
    just: bool,
) -> bool {
    pad.is_some_and(|pad| {
        if just {
            pad.just_pressed(button)
        } else {
            pad.pressed(button)
        }
    })
}

pub(crate) struct Presenter<'a> {
    pub(crate) sounds: Option<&'a mut Sounds>,
    pub(crate) packs: &'a PackStack,
    pub(crate) atlas: &'a Atlas,
    pub(crate) mining: &'a mut Mining,
    pub(crate) scene: &'a HandcraftedScene,
    pub(crate) origin: [f64; 3],
    pub(crate) ui: &'a mut frame::MinecraftUi,
    pub(crate) inventory_ui: &'a InventoryUi,
}

pub(crate) fn play(effects: Vec<Effect>, presenter: &mut Presenter<'_>) {
    let mut loaded = None;
    for effect in effects {
        match effect {
            Effect::Sound {
                event,
                at,
                volume,
                pitch,
                spread,
            } => {
                if let Some(sounds) = presenter.sounds.as_deref_mut() {
                    let pitch = if spread == 0.0 {
                        pitch
                    } else {
                        pitch + sounds.random() * spread
                    };
                    let at =
                        at.map(|at| Vec3::from_array(sim::voxel::to_map(presenter.origin, at)));
                    sounds.play(presenter.packs, &event, at, volume, pitch);
                }
            }
            Effect::Crumbs {
                item,
                eye,
                yaw,
                pitch,
                count,
            } => {
                if let Some(texture) = minecraft_terrain::pack::ResourceId::parse(&item)
                    .ok()
                    .and_then(|id| {
                        minecraft_terrain::model::item_particle_texture(presenter.packs, &id)
                            .ok()
                            .flatten()
                    })
                    .filter(|texture| presenter.atlas.contains(texture))
                {
                    presenter.mining.eat_burst(
                        presenter.packs,
                        eye.as_vec3(),
                        yaw,
                        pitch,
                        count,
                        &texture,
                    );
                }
            }
            Effect::HitParticle { pos, face, block } => {
                presenter.mining.hit_particle(
                    presenter.packs,
                    presenter.scene,
                    presenter.atlas,
                    (pos, face),
                    &block,
                );
            }
            Effect::Chat(line) => {
                let language = language(presenter.inventory_ui, presenter.packs, &mut loaded);
                crate::minecraft_chat::show(&mut presenter.ui.chat, &[line], language);
            }
            Effect::Overlay(key) => {
                let language = language(presenter.inventory_ui, presenter.packs, &mut loaded);
                crate::minecraft_respawn::show_overlay(&mut *presenter.ui, language, &key);
            }
        }
    }
}

fn language<'a>(
    inventory_ui: &'a InventoryUi,
    packs: &PackStack,
    loaded: &'a mut Option<HashMap<String, String>>,
) -> &'a HashMap<String, String> {
    match inventory_ui.language() {
        Some(language) => language,
        None => loaded.get_or_insert_with(|| {
            minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new())
        }),
    }
}
