use std::collections::HashMap;

use bevy::prelude::*;
use frame::MinecraftUi;
use frame::minecraft_ui::McOverlayMessage;
use minecraft_commands::{Clientbound, Text};
use minecraft_survival::entities::Entities;
use minecraft_survival::level::Terrain;
use minecraft_survival::player_nbt::Respawn;
use minecraft_survival::respawn_block::{
    self, BAD_RESPAWN_POINT_POWER, BedUse, BlockView, OCCUPIED, Pos, SPAWN_SET, Sleeper,
};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::{Block, Scene};
use minecraft_terrain::server::Blast;
use minecraft_terrain::terrain::Dimension;
use minecraftoss_core::{BlockStateId, Registries};

use crate::minecraft_inventory::InventoryUi;
use crate::minecraft_sounds::Sounds;

const BLOCK_REACH: f64 = 5.0;
const DARK_SKY_LIGHT: f32 = 11.0;
const TICKS_PER_SECOND: f64 = 20.0;

struct SceneView<'a>(&'a Terrain);

impl BlockView for SceneView<'_> {
    fn state_at(&mut self, pos: Pos) -> BlockStateId {
        Scene::block(&self.0.scene, pos)
            .and_then(|block| self.0.stream.states.state_of(block))
            .unwrap_or(BlockStateId::AIR)
    }
}

#[derive(Default)]
pub(crate) struct RespawnBlockUse {
    pub(crate) writes: Vec<(Pos, Option<Block>)>,
    blast: Option<Blast>,
    sounds: Vec<(&'static str, [f64; 3])>,
    respawn: Option<Respawn>,
    overlay: Option<String>,
}

pub(crate) fn use_block(
    aim: &minecraftoss_player::Player,
    terrain: &Terrain,
    registries: &Registries,
    dimension: Dimension,
) -> Option<RespawnBlockUse> {
    let pos = aim
        .target(
            &minecraft_terrain::doors::Outlined(&terrain.scene),
            BLOCK_REACH,
        )?
        .pos;
    let mut used = RespawnBlockUse::default();
    let mut view = SceneView(terrain);
    match respawn_block::use_bed(&mut view, registries, &terrain.rules, pos)? {
        BedUse::Destroy {
            removed,
            blast,
            sound,
            message,
        } => {
            let head = respawn_block::centre(removed[0]);
            used.writes = removed.into_iter().map(|at| (at, None)).collect();
            used.blast = blast.map(|center| Blast {
                center,
                power: BAD_RESPAWN_POINT_POWER,
                fire: true,
            });
            used.sounds.extend(sound.map(|event| (event, head)));
            used.overlay = message;
        }
        BedUse::Occupied => used.overlay = Some(OCCUPIED.to_owned()),
        BedUse::Sleep(bed) => {
            let sleeper = Sleeper {
                feet: aim.pos.to_array(),
                yaw: aim.yaw as f32,
                pitch: aim.pitch as f32,
                dimension,
                dark: terrain.environment.sky_light_level() <= DARK_SKY_LIGHT,
            };
            match respawn_block::rest_in_bed(&mut view, registries, &terrain.rules, &bed, &sleeper)
            {
                Ok(rest) => {
                    used.respawn = rest.respawn;
                    used.overlay = rest.problem;
                }
                Err(problem) => used.overlay = Some(problem.to_owned()),
            }
        }
        BedUse::Incomplete => {}
    }
    Some(used)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn finish(
    used: RespawnBlockUse,
    entities: &mut Entities,
    respawn: &mut Option<Respawn>,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    origin: [f64; 3],
    ui: &mut MinecraftUi,
    inventory_ui: &InventoryUi,
) {
    if let Some(blast) = used.blast {
        entities.explode_respawn_point(blast);
    }
    if let Some(sounds) = sounds {
        for (event, at) in used.sounds {
            let at = Vec3::from_array(sim::voxel::to_map(origin, at));
            sounds.play(packs, event, Some(at), 1.0, 1.0);
        }
    }
    let loaded;
    let language = match inventory_ui.language() {
        Some(language) => language,
        None => {
            loaded = minecraft_terrain::item_icons::language(packs).unwrap_or_default();
            &loaded
        }
    };
    if let Some(point) = used.respawn
        && respawn_block::set_respawn(respawn, point)
    {
        let line = Clientbound::System {
            text: Text::translate(SPAWN_SET, Vec::new()),
            overlay: false,
        };
        crate::minecraft_chat::show(&mut ui.chat, &[line], language);
    }
    if let Some(key) = used.overlay {
        show_overlay(ui, language, &key);
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
