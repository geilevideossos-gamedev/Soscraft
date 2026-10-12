use std::collections::HashMap;

use bevy::prelude::*;
use frame::MinecraftUi;
use frame::minecraft_ui::McOverlayMessage;
use minecraft_commands::{Clientbound, Text};
use minecraft_survival::entities::Entities;
use minecraft_survival::level::Terrain;
use minecraft_survival::player_nbt::Respawn;
use minecraft_survival::respawn_block::{
    self, ANCHOR_CHARGE_SOUND, ANCHOR_SET_SPAWN_SOUND, AnchorUse, BAD_RESPAWN_POINT_POWER, BedUse,
    Bed, BlockView, Fuel, GLOWSTONE, OCCUPIED, Outcome, Pos, SPAWN_SET, Sleeper,
};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::{Block, Scene};
use minecraft_terrain::server::Blast;
use minecraft_terrain::terrain::Dimension;
use minecraftoss_core::{BlockStateId, Registries};
use minecraftoss_player::inventory::Inventory;

use crate::minecraft_inventory::InventoryUi;
use crate::minecraft_sounds::Sounds;

const BLOCK_REACH: f64 = 5.0;
const TICKS_PER_SECOND: f64 = 20.0;

struct SceneView<'a>(&'a Terrain);

impl BlockView for SceneView<'_> {
    fn state_at(&mut self, pos: Pos) -> BlockStateId {
        Scene::block(&self.0.scene, pos)
            .and_then(|block| self.0.stream.states.state_of(block))
            .unwrap_or(BlockStateId::AIR)
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Hand<'a> {
    pub(crate) main: bool,
    pub(crate) held: Option<&'a str>,
    pub(crate) offhand: Option<&'a str>,
}

pub(crate) struct Night<'a> {
    pub(crate) dark: bool,
    pub(crate) creative: bool,
    pub(crate) monsters: &'a dyn Fn([f64; 6]) -> bool,
}

#[derive(Default)]
pub(crate) struct RespawnBlockUse {
    pub(crate) writes: Vec<(Pos, Option<Block>)>,
    blast: Option<(Blast, Option<Pos>)>,
    sounds: Vec<(&'static str, [f64; 3])>,
    respawn: Option<(Respawn, Option<[f64; 3]>)>,
    overlay: Option<String>,
    spends: bool,
    occupied: Option<Pos>,
    sleep: Option<Bed>,
}

fn bad_respawn_point(center: [f64; 3]) -> Blast {
    Blast {
        center,
        power: BAD_RESPAWN_POINT_POWER,
        fire: true,
    }
}

pub(crate) fn use_block(
    aim: &minecraftoss_player::Player,
    terrain: &Terrain,
    registries: &Registries,
    dimension: Dimension,
    hand: Hand,
    night: Night,
) -> Option<RespawnBlockUse> {
    let pos = aim
        .target(
            &minecraft_terrain::doors::Outlined(&terrain.scene),
            BLOCK_REACH,
        )?
        .pos;
    let mut view = SceneView(terrain);
    let fuel = Fuel {
        main: hand.main,
        held: hand.held == Some(GLOWSTONE),
        offhand: hand.main && hand.offhand == Some(GLOWSTONE),
    };
    if let Some(anchor) =
        respawn_block::use_anchor(&mut view, registries, &terrain.rules, pos, dimension, fuel)
    {
        return Some(use_anchor(anchor, terrain));
    }
    if !hand.main {
        return None;
    }
    let mut used = RespawnBlockUse::default();
    match respawn_block::use_bed(&mut view, registries, &terrain.rules, pos)? {
        BedUse::Destroy {
            removed,
            blast,
            sound,
            message,
        } => {
            let head = respawn_block::centre(removed[0]);
            used.writes = removed.into_iter().map(|at| (at, None)).collect();
            used.blast = blast.map(|center| (bad_respawn_point(center), None));
            used.sounds.extend(sound.map(|event| (event, head)));
            used.overlay = message;
        }
        BedUse::Occupied(head) => used.occupied = Some(head),
        BedUse::Sleep(bed) => {
            let sleeper = Sleeper {
                feet: aim.pos.to_array(),
                yaw: aim.yaw as f32,
                pitch: aim.pitch as f32,
                dimension,
                dark: night.dark,
                creative: night.creative,
            };
            let rest = respawn_block::rest_in_bed(
                &mut view,
                registries,
                &terrain.rules,
                &bed,
                &sleeper,
                night.monsters,
            );
            used.respawn = rest.respawn.map(|point| (point, None));
            match rest.outcome {
                Outcome::Problem(problem) => used.overlay = problem,
                Outcome::Sleep => used.sleep = Some(bed),
            }
        }
        BedUse::Incomplete => {}
    }
    Some(used)
}

fn use_anchor(anchor: AnchorUse, terrain: &Terrain) -> RespawnBlockUse {
    let mut used = RespawnBlockUse::default();
    match anchor {
        AnchorUse::Charge { pos, state } => {
            used.writes = vec![(pos, terrain.stream.states.block(state).cloned())];
            used.sounds
                .push((ANCHOR_CHARGE_SOUND, respawn_block::centre(pos)));
            used.spends = true;
        }
        AnchorUse::SetSpawn(point) => {
            used.respawn = Some((point, Some(respawn_block::centre(point.at.pos))));
        }
        AnchorUse::Explode { pos, shielded } => {
            used.writes = vec![(pos, None)];
            used.blast = Some((
                bad_respawn_point(respawn_block::centre(pos)),
                shielded.then_some(pos),
            ));
        }
    }
    used
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish(
    used: RespawnBlockUse,
    entities: &mut Entities,
    respawn: &mut Option<Respawn>,
    inventory: &mut Inventory,
    slot: usize,
    consumes: bool,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    origin: [f64; 3],
    ui: &mut MinecraftUi,
    inventory_ui: &InventoryUi,
) -> Option<Bed> {
    if let Some((blast, shield)) = used.blast {
        entities.explode_respawn_point(blast, shield);
    }
    if used.spends
        && consumes
        && let Some(stack) = inventory.slots.get_mut(slot)
        && let Some(held) = stack.as_mut()
    {
        held.count -= 1;
        if held.count == 0 {
            *stack = None;
        }
    }
    let mut heard = used.sounds;
    let loaded;
    let language = match inventory_ui.language() {
        Some(language) => language,
        None => {
            loaded =
                minecraft_terrain::item_icons::language(packs).unwrap_or_else(|_| HashMap::new());
            &loaded
        }
    };
    if let Some((point, sound)) = used.respawn
        && respawn_block::set_respawn(respawn, point)
    {
        let line = Clientbound::System {
            text: Text::translate(SPAWN_SET, Vec::new()),
            overlay: false,
        };
        crate::minecraft_chat::show(&mut ui.chat, &[line], language);
        heard.extend(sound.map(|at| (ANCHOR_SET_SPAWN_SOUND, at)));
    }
    if let Some(key) = used.overlay {
        show_overlay(ui, language, &key);
    }
    if let Some(head) = used.occupied
        && !entities.wake_villager_in(head)
    {
        show_overlay(ui, language, OCCUPIED);
    }
    if let Some(sounds) = sounds {
        for (event, at) in heard {
            let at = Vec3::from_array(sim::voxel::to_map(origin, at));
            sounds.play(packs, event, Some(at), 1.0, 1.0);
        }
    }
    used.sleep
}

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
