use bevy::prelude::*;
use minecraft_survival::entities::Entities;
use minecraft_survival::level::Terrain;
use minecraft_survival::respawn_block::{self, BAD_RESPAWN_POINT_POWER, BedUse, BlockView, Pos};
use minecraft_terrain::pack::PackStack;
use minecraft_terrain::scene::{Block, Scene};
use minecraft_terrain::server::Blast;
use minecraftoss_core::{BlockStateId, Registries};

use crate::minecraft_sounds::Sounds;

const BLOCK_REACH: f64 = 5.0;

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
}

pub(crate) fn use_block(
    aim: &minecraftoss_player::Player,
    terrain: &Terrain,
    registries: &Registries,
) -> Option<RespawnBlockUse> {
    let pos = aim
        .target(
            &minecraft_terrain::doors::Outlined(&terrain.scene),
            BLOCK_REACH,
        )?
        .pos;
    let mut used = RespawnBlockUse::default();
    match respawn_block::use_bed(&mut SceneView(terrain), registries, &terrain.rules, pos)? {
        BedUse::Destroy {
            removed,
            blast,
            sound,
            ..
        } => {
            let head = respawn_block::centre(removed[0]);
            used.writes = removed.into_iter().map(|at| (at, None)).collect();
            used.blast = blast.map(|center| Blast {
                center,
                power: BAD_RESPAWN_POINT_POWER,
                fire: true,
            });
            used.sounds.extend(sound.map(|event| (event, head)));
        }
        BedUse::Incomplete | BedUse::Occupied | BedUse::Sleep(_) => {}
    }
    Some(used)
}

pub(crate) fn finish(
    used: RespawnBlockUse,
    entities: &mut Entities,
    sounds: Option<&mut Sounds>,
    packs: &PackStack,
    origin: [f64; 3],
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
}
