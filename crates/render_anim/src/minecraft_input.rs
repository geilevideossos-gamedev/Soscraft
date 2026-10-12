use std::collections::HashMap;

use bevy::input::gamepad::GamepadButton;
use bevy::prelude::*;
use minecraft_net::messages::{HandEvent, HandEventKind, HandState, Target};
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

const STATE_EVERY: f64 = 0.05;

#[derive(Default)]
pub(crate) struct HandSampler {
    seq: u32,
    tick: u32,
    attack: bool,
    using: bool,
    sent_at: Option<f64>,
}

#[derive(Debug, Default, PartialEq)]
pub(crate) struct Sampled {
    pub(crate) events: Vec<HandEvent>,
    pub(crate) state: Option<HandState>,
}

impl HandSampler {
    pub(crate) fn sample(
        &mut self,
        hands: Hands,
        swap: bool,
        drop: Option<bool>,
        target: Target,
        seconds: f64,
    ) -> Sampled {
        let live = !hands.captured;
        let mut kinds = Vec::new();
        if live && hands.attack_pressed {
            kinds.push(HandEventKind::AttackPressed);
            self.attack = true;
        }
        if self.attack && !(live && hands.attack_held) {
            kinds.push(HandEventKind::AttackReleased);
            self.attack = false;
        }
        if live && hands.use_pressed {
            kinds.push(HandEventKind::UsePressed);
            self.using = true;
        }
        if self.using && !(live && hands.use_held) {
            kinds.push(HandEventKind::UseReleased);
            self.using = false;
        }
        if live && swap {
            kinds.push(HandEventKind::SwapHands);
        }
        if let Some(whole_stack) = drop.filter(|_| live) {
            kinds.push(HandEventKind::Drop { whole_stack });
        }
        let events = kinds
            .into_iter()
            .map(|kind| {
                self.seq = self.seq.wrapping_add(1);
                HandEvent {
                    seq: self.seq,
                    kind,
                    target,
                }
            })
            .collect();
        let holding = self.attack || self.using;
        if !holding {
            self.sent_at = None;
        }
        let due = holding && self.sent_at.is_none_or(|at| seconds - at >= STATE_EVERY);
        let state = due.then(|| {
            self.sent_at = Some(seconds);
            self.tick = self.tick.wrapping_add(1);
            HandState {
                tick: self.tick,
                attack: self.attack,
                use_item: self.using,
                target,
            }
        });
        Sampled { events, state }
    }
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
            Effect::Dig {
                pos,
                face,
                block,
                sound,
            } => {
                if let (Some(sounds), Some((event, volume, pitch))) =
                    (presenter.sounds.as_deref_mut(), sound)
                {
                    let centre = [
                        f64::from(pos.0) + 0.5,
                        f64::from(pos.1) + 0.5,
                        f64::from(pos.2) + 0.5,
                    ];
                    let at = Vec3::from_array(sim::voxel::to_map(presenter.origin, centre));
                    sounds.play(presenter.packs, &event, Some(at), volume, pitch);
                }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(sampled: &Sampled) -> Vec<HandEventKind> {
        sampled.events.iter().map(|event| event.kind).collect()
    }

    #[test]
    fn a_click_sends_a_press_and_a_release_and_a_hold_keeps_its_state_coming() {
        let mut sampler = HandSampler::default();
        let click = Hands {
            attack_pressed: true,
            ..Hands::default()
        };
        let clicked = sampler.sample(click, false, None, Target::Miss, 0.0);
        assert_eq!(
            kinds(&clicked),
            [HandEventKind::AttackPressed, HandEventKind::AttackReleased]
        );
        assert_eq!(clicked.state, None);
        let press = Hands {
            use_pressed: true,
            use_held: true,
            ..Hands::default()
        };
        let held = Hands {
            use_held: true,
            ..Hands::default()
        };
        let pressed = sampler.sample(press, false, None, Target::Miss, 1.0);
        assert_eq!(kinds(&pressed), [HandEventKind::UsePressed]);
        assert!(
            pressed
                .state
                .is_some_and(|state| state.use_item && !state.attack)
        );
        assert_eq!(
            sampler.sample(held, false, None, Target::Miss, 1.01).state,
            None
        );
        let later = sampler.sample(held, false, None, Target::Miss, 1.06);
        assert!(later.events.is_empty());
        assert!(
            later
                .state
                .is_some_and(|state| state.tick > pressed.state.unwrap().tick)
        );
        let chatting = Hands {
            captured: true,
            ..held
        };
        let opened = sampler.sample(chatting, true, Some(true), Target::Miss, 1.2);
        assert_eq!(kinds(&opened), [HandEventKind::UseReleased]);
        let seqs: Vec<u32> = clicked
            .events
            .iter()
            .chain(&pressed.events)
            .chain(&opened.events)
            .map(|event| event.seq)
            .collect();
        assert_eq!(seqs, [1, 2, 3, 4]);
        let swapped = sampler.sample(Hands::default(), true, Some(false), Target::Miss, 2.0);
        assert_eq!(
            kinds(&swapped),
            [
                HandEventKind::SwapHands,
                HandEventKind::Drop { whole_stack: false }
            ]
        );
    }
}
