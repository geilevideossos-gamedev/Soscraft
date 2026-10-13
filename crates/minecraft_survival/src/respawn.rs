use minecraft_net::Dimension;
use minecraft_terrain::game_mode::GameMode;
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

pub fn level_spawn(globals: &Globals, level: &mut Level) -> Place {
    let spawn = &globals.spawn;
    let home = spawn_dimension(spawn) == level.dimension;
    let (yaw, pitch) = if home {
        (spawn.yaw, spawn.pitch)
    } else {
        (0.0, 0.0)
    };
    let adventure = globals
        .save
        .as_ref()
        .is_some_and(|save| save.default_mode().mode == GameMode::Adventure);
    let (x, y, z) = if home && !adventure && !level.terrain.stream.is_remote() {
        level
            .terrain
            .stream
            .respawn_position(globals.game_rules.int("respawn_radius"))
    } else {
        level.terrain.stream.player_spawn
    };
    Place::standing(level.dimension, [x, y, z], yaw, pitch)
}

pub fn respawn_point(player: &Player) -> Option<Respawn> {
    player.respawn
}

pub fn respawn_in(
    globals: &Globals,
    level: &mut Level,
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
                .unwrap_or_else(|| level_spawn(globals, level)),
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

    fn player(chat: &mut minecraft_commands::CommandServer, id: u32, name: &str) -> Player {
        let id = minecraft_commands::PlayerId(id);
        chat.join(id, name);
        Player::new(
            chat.profile(id).unwrap(),
            minecraft_terrain::game_mode::PlayerMode::default(),
            crate::survival::Survival::default(),
            minecraftoss_player::inventory::Inventory::default(),
        )
    }

    fn level_and_globals() -> Option<(Level, Globals)> {
        let level =
            crate::commands::tests::test_stream().and_then(crate::commands::tests::test_level)?;
        let registries = level.terrain.stream.states.registries().clone();
        Some((level, crate::server::tests::test_globals(registries, None)))
    }

    #[test]
    fn each_player_respawns_at_their_own_point() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let mut friend = player(&mut chat, 1, "Alex");
        host.respawn = Some(point(Dimension::Overworld, (40, 70, 40), true));
        friend.respawn = Some(point(Dimension::Overworld, (-30, 80, 12), true));
        assert!(!respawn(&globals, &mut level, &mut friend));
        assert!(!respawn(&globals, &mut level, &mut host));
        let feet = |player: &Player| player.arrival.map(|place| place.feet);
        assert_eq!(feet(&host), Some([40.5, 70.0, 40.5]));
        assert_eq!(feet(&friend), Some([-29.5, 80.0, 12.5]));
        assert!(host.switching.is_none() && friend.switching.is_none());
    }

    fn spawn_column(level: &Level) -> (f64, f64) {
        let (x, _, z) = level.terrain.stream.world_spawn;
        (f64::from(x) + 0.5, f64::from(z) + 0.5)
    }

    #[test]
    fn bedless_respawns_land_within_the_respawn_radius() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        assert_eq!(globals.game_rules.int("respawn_radius"), 10);
        let (x, z) = spawn_column(&level);
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let mut steve = player(&mut chat, 0, "Steve");
        for _ in 0..4 {
            assert!(!respawn_in(&globals, &mut level, &mut steve, |_| None));
            let arrival = steve.arrival.take().expect("a bedless respawn arrives");
            assert_eq!(
                (arrival.dimension, arrival.yaw, arrival.pitch),
                (Dimension::Overworld, globals.spawn.yaw, globals.spawn.pitch)
            );
            assert!(
                (arrival.feet[0] - x).abs() <= 10.0 && (arrival.feet[2] - z).abs() <= 10.0,
                "within 10 blocks of the world spawn: {arrival:?}"
            );
            assert!(steve.switching.is_none());
        }
    }

    #[test]
    fn respawn_radius_zero_respawns_on_the_spawn_column() {
        let Some((mut level, mut globals)) = level_and_globals() else {
            return;
        };
        globals.game_rules.set("respawn_radius", "0").unwrap();
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let mut steve = player(&mut chat, 0, "Steve");
        let mut alex = player(&mut chat, 1, "Alex");
        assert!(!respawn_in(&globals, &mut level, &mut steve, |_| None));
        assert!(!respawn_in(&globals, &mut level, &mut alex, |_| None));
        let column = |player: &Player| player.arrival.map(|place| (place.feet[0], place.feet[2]));
        assert_eq!(column(&steve), Some(spawn_column(&level)));
        assert_eq!(column(&alex), Some(spawn_column(&level)));
    }

    #[test]
    fn a_friends_respawn_in_another_dimension_sets_off_their_own_switch() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let mut friend = player(&mut chat, 1, "Alex");
        friend.respawn = Some(point(Dimension::Nether, (10, 70, 10), true));
        assert!(!respawn(&globals, &mut level, &mut host));
        assert!(host.switching.is_none(), "the host respawns in this level");
        assert!(!respawn(&globals, &mut level, &mut friend));
        assert_eq!(
            friend.switching.map(|switch| (switch.to, switch.respawn)),
            Some((Dimension::Nether, true))
        );
    }
}
