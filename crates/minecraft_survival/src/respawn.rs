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
    let radius = globals.game_rules.int("respawn_radius");
    let (x, y, z) = spawn_position(&mut level.terrain.stream, home, adventure, radius);
    Place::standing(level.dimension, [x, y, z], yaw, pitch)
}

fn spawn_position(
    stream: &mut minecraft_terrain::terrain::TerrainStream,
    home: bool,
    adventure: bool,
    radius: i32,
) -> (f64, f64, f64) {
    if !home || stream.is_remote() {
        stream.player_spawn
    } else if adventure {
        stream.exact_spawn_position()
    } else {
        stream.respawn_position(radius)
    }
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

pub fn flare_stands(flare: &Place, open: impl Fn((i32, i32, i32)) -> Option<bool>) -> bool {
    let [x, y, z] = flare.feet;
    let (x, z) = (x.floor() as i32, z.floor() as i32);
    [0.5, 1.5]
        .into_iter()
        .all(|up| open((x, (y + up).floor() as i32, z)) != Some(false))
}

pub fn respawn_at_flare(globals: &Globals, level: &Level, player: &mut Player) -> bool {
    let Some(flare) = player.arsenal.take_flare() else {
        return false;
    };
    if flare.dimension == level.dimension {
        let open = |pos| crate::level::open_for_respawn(&level.terrain, &globals.registries, pos);
        if !flare_stands(&flare, open) {
            return false;
        }
        player.arrival = Some(flare);
    } else {
        let reason = crate::switch::loading_reason(true, level.dimension, flare.dimension);
        player.switch_to(flare.dimension, Some(flare), reason);
    }
    player.respawning = None;
    true
}

pub fn respawn(globals: &Globals, level: &mut Level, player: &mut Player) -> bool {
    if respawn_at_flare(globals, level, player) {
        return false;
    }
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

    #[test]
    fn a_flare_stands_where_the_body_fits() {
        let flare = Place::standing(Dimension::Overworld, [3.5, 64.0, -2.5], 0.0, 0.0);
        assert!(flare_stands(&flare, |_| Some(true)));
        assert!(flare_stands(&flare, |_| None), "an unloaded spot is not refused");
        assert!(!flare_stands(&flare, |pos| Some(pos != (3, 65, -3))), "a block at the head");
        assert!(!flare_stands(&flare, |pos| Some(pos != (3, 64, -3))), "a block at the feet");
        assert!(flare_stands(&flare, |pos| Some(pos != (3, 63, -3))), "the floor");
        let on_a_slab = Place::standing(Dimension::Overworld, [3.5, 64.5, -2.5], 0.0, 0.0);
        assert!(flare_stands(&on_a_slab, |pos| Some(pos != (3, 64, -3))));
    }

    #[test]
    fn a_flare_wins_over_the_respawn_point_once_for_its_own_player() {
        let Some((mut level, globals)) = level_and_globals() else {
            return;
        };
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let mut host = player(&mut chat, 0, "Steve");
        let mut friend = player(&mut chat, 1, "Alex");
        host.respawn = Some(point(Dimension::Overworld, (40, 70, 40), true));
        friend.respawn = Some(point(Dimension::Overworld, (-30, 80, 12), true));
        let flare = Place::standing(Dimension::Overworld, [5.25, 300.0, -7.75], 30.0, 0.0);
        friend.arsenal.set_flare(Some(flare));
        let feet = |player: &Player| player.arrival.map(|place| place.feet);
        assert!(!respawn(&globals, &mut level, &mut host));
        assert!(!respawn(&globals, &mut level, &mut friend));
        assert_eq!(feet(&host), Some([40.5, 70.0, 40.5]), "the friend's flare does not move the host");
        assert_eq!(friend.arrival, Some(flare), "the flare wins over the friend's point");
        assert_eq!(friend.arsenal.flare(), None, "and is used up");
        assert!(friend.respawn.is_some(), "the point stays");
        friend.arrival = None;
        assert!(!respawn(&globals, &mut level, &mut friend));
        assert_eq!(feet(&friend), Some([-29.5, 80.0, 12.5]), "the next death respawns at the point");
        let nether = Place::standing(Dimension::Nether, [1.5, 70.0, 2.5], 0.0, 0.0);
        host.arsenal.set_flare(Some(nether));
        host.respawning = Some(Dimension::Overworld);
        assert!(!respawn(&globals, &mut level, &mut host));
        assert_eq!(
            host.switching.map(|switch| (switch.to, switch.arrival, switch.respawn)),
            Some((Dimension::Nether, Some(nether), false)),
            "a flare in another dimension switches there and stands at it"
        );
        assert_eq!(host.respawning, None);
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
    fn adventure_worlds_respawn_on_the_spawn_column_whatever_the_radius() {
        let Some((mut level, _)) = level_and_globals() else {
            return;
        };
        let (x, z) = spawn_column(&level);
        let stream = &mut level.terrain.stream;
        let exact = stream.exact_spawn_position();
        assert_eq!((exact.0, exact.2), (x, z));
        for _ in 0..4 {
            assert_eq!(spawn_position(stream, true, true, 10), exact);
        }
        assert_eq!(spawn_position(stream, false, true, 10), stream.player_spawn);
        let spread = spawn_position(stream, true, false, 10);
        assert!((spread.0 - x).abs() <= 10.0 && (spread.2 - z).abs() <= 10.0);
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
