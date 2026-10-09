use minecraft_net::Dimension;
use minecraft_terrain::level_storage::RespawnData;

use crate::level::Level;
use crate::player::{Player, respawn_place};
use crate::player_nbt::{Place, Respawn};
use crate::save::spawn_dimension;
use crate::server::Globals;
use crate::switch::LoadingReason;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Respawned {
    pub dimension: Dimension,
    pub place: Option<Place>,
    pub missing: bool,
}

pub fn respawn_dimension(spawn: &RespawnData, respawn: Option<&Respawn>) -> Dimension {
    respawn.map_or_else(|| spawn_dimension(spawn), |respawn| respawn.at.dimension)
}

pub fn respawn_target(
    spawn: &RespawnData,
    respawn: Option<&Respawn>,
    open: impl Fn((i32, i32, i32)) -> Option<bool>,
    spawn_block: impl FnOnce(&Respawn) -> Option<Place>,
) -> Respawned {
    let world_spawn = Respawned {
        dimension: spawn_dimension(spawn),
        place: None,
        missing: false,
    };
    let Some(respawn) = respawn else {
        return world_spawn;
    };
    match respawn_place(respawn, open, spawn_block) {
        Some(place) => Respawned {
            dimension: respawn.at.dimension,
            place: Some(place),
            missing: false,
        },
        None => Respawned {
            missing: true,
            ..world_spawn
        },
    }
}

pub fn level_spawn(spawn: &RespawnData, level: &Level) -> Place {
    let (x, y, z) = level.terrain.stream.player_spawn;
    let (yaw, pitch) = if spawn_dimension(spawn) == level.dimension {
        (spawn.yaw, spawn.pitch)
    } else {
        (0.0, 0.0)
    };
    Place::standing(level.dimension, [x, y, z], yaw, pitch)
}

pub fn respawn_point(player: &Player) -> Option<Respawn> {
    player.respawn
}

pub fn respawn_in(
    globals: &Globals,
    level: &Level,
    player: &mut Player,
    spawn_block: impl FnOnce(&Respawn) -> Option<Place>,
) -> bool {
    let open = |pos| crate::level::open_for_respawn(&level.terrain, &globals.registries, pos);
    let point = respawn_point(player);
    let respawned = respawn_target(&globals.spawn, point.as_ref(), open, spawn_block);
    if respawned.missing {
        player.respawn = None;
    }
    if respawned.dimension == level.dimension {
        player.arrival = Some(
            respawned
                .place
                .unwrap_or_else(|| level_spawn(&globals.spawn, level)),
        );
    } else {
        player.switch_to(respawned.dimension, respawned.place, LoadingReason::Other);
    }
    respawned.missing
}

pub fn respawn(globals: &Globals, level: &mut Level, player: &mut Player) -> bool {
    let target = player
        .respawning
        .take()
        .unwrap_or_else(|| respawn_dimension(&globals.spawn, respawn_point(player).as_ref()));
    if target == level.dimension {
        return crate::respawn_block::respawn_at_block(globals, level, player);
    }
    player.switch_to(
        target,
        None,
        crate::switch::loading_reason(true, level.dimension, target),
    );
    if let Some(switch) = player.switching.as_mut() {
        switch.respawn = true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::player_nbt::GlobalPos;

    fn point(dimension: Dimension, pos: (i32, i32, i32), forced: bool) -> Respawn {
        Respawn {
            at: GlobalPos { dimension, pos },
            yaw: 45.0,
            pitch: 5.0,
            forced,
        }
    }

    #[test]
    fn the_respawn_dimension_is_the_point_s_or_else_the_world_spawn_s() {
        let overworld = crate::save::world_spawn(Dimension::Overworld, (8, 64, -4));
        assert_eq!(respawn_dimension(&overworld, None), Dimension::Overworld);
        let anchor = point(Dimension::Nether, (10, 70, 10), true);
        assert_eq!(
            respawn_dimension(&overworld, Some(&anchor)),
            Dimension::Nether
        );
        let nether = crate::save::world_spawn(Dimension::Nether, (0, 70, 0));
        assert_eq!(respawn_dimension(&nether, None), Dimension::Nether);
    }

    #[test]
    fn a_respawn_goes_to_the_point_or_else_the_world_spawn_and_says_when_the_point_is_gone() {
        let spawn = crate::save::world_spawn(Dimension::Overworld, (8, 64, -4));
        assert_eq!(
            respawn_target(&spawn, None, |_| Some(true), |_| None),
            Respawned {
                dimension: Dimension::Overworld,
                place: None,
                missing: false
            }
        );
        let anchor = point(Dimension::Nether, (10, 70, 10), true);
        let kept = respawn_target(&spawn, Some(&anchor), |_| Some(true), |_| None);
        assert_eq!(kept.dimension, Dimension::Nether);
        assert!(!kept.missing);
        assert_eq!(
            kept.place,
            Some(Place::standing(
                Dimension::Nether,
                [10.5, 70.0, 10.5],
                45.0,
                5.0
            ))
        );
        let gone = respawn_target(&spawn, Some(&anchor), |_| Some(false), |_| None);
        assert_eq!(
            gone,
            Respawned {
                dimension: Dimension::Overworld,
                place: None,
                missing: true
            },
            "missingRespawnBlock falls back to the world spawn"
        );
        let bed = point(Dimension::Overworld, (1, 64, 1), false);
        let beside = Place::standing(Dimension::Overworld, [2.5, 64.0, 1.5], 90.0, 0.0);
        let slept = respawn_target(&spawn, Some(&bed), |_| Some(true), |_| Some(beside));
        assert_eq!(slept.place, Some(beside));
    }
}
