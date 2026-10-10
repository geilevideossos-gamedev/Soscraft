pub mod dragon_fight;

use std::any::Any;

use minecraft_net::Dimension;
use minecraft_terrain::level_storage::RespawnData;
use minecraft_terrain::scene::Scene;
use minecraft_terrain::server::{Job, ServerSim};

use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::player_nbt::{Place, Respawn};
use crate::respawn::{respawn_dimension, respawn_in, respawn_point};
use crate::server::Globals;
use crate::switch::loading_reason;

pub const END_PORTAL: &str = "minecraft:end_portal";
pub const END_SPAWN_POINT: (i32, i32, i32) = (100, 50, 0);
pub const WEST_YAW: f32 = 90.0;
const PLAYER_WIDTH: f64 = 0.6;
const PLAYER_HEIGHT: f64 = 1.8;
const INSIDE_DEFLATE: f64 = 1.0e-5;
const PORTAL_SHAPE: (f64, f64) = (6.0 / 16.0, 12.0 / 16.0);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EndTravel {
    pub platform: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trip {
    IntoTheEnd,
    OutOfTheEnd(Dimension),
}

pub fn touched_end_portal(
    feet: [f64; 3],
    is_portal: impl Fn((i32, i32, i32)) -> bool,
) -> Option<(i32, i32, i32)> {
    let half = PLAYER_WIDTH / 2.0 - INSIDE_DEFLATE;
    let low = [feet[0] - half, feet[1] + INSIDE_DEFLATE, feet[2] - half];
    let high = [
        feet[0] + half,
        feet[1] + PLAYER_HEIGHT - INSIDE_DEFLATE,
        feet[2] + half,
    ];
    let range = |axis: usize| low[axis].floor() as i32..=high[axis].floor() as i32;
    for z in range(2) {
        for y in range(1) {
            for x in range(0) {
                let bottom = f64::from(y) + PORTAL_SHAPE.0;
                let top = f64::from(y) + PORTAL_SHAPE.1;
                if low[1] < top && high[1] > bottom && is_portal((x, y, z)) {
                    return Some((x, y, z));
                }
            }
        }
    }
    None
}

pub fn is_end_portal(terrain: &Terrain, pos: (i32, i32, i32)) -> bool {
    Scene::block(&terrain.scene, pos).is_some_and(|block| block.id.key() == END_PORTAL)
}

pub fn end_arrival(from: Option<&Place>) -> Place {
    let (x, y, z) = END_SPAWN_POINT;
    Place {
        motion: from.map_or([0.0; 3], |place| place.motion),
        on_ground: false,
        ..Place::standing(
            Dimension::End,
            [f64::from(x) + 0.5, f64::from(y) - 1.0, f64::from(z) + 0.5],
            WEST_YAW,
            from.map_or(0.0, |place| place.pitch),
        )
    }
}

pub fn platform_origin() -> (i32, i32, i32) {
    let (x, y, z) = END_SPAWN_POINT;
    (x, y - 1, z)
}

pub fn platform_chunks() -> Vec<(i32, i32)> {
    let (x, _, z) = platform_origin();
    let mut chunks = Vec::new();
    for cz in (z - 2).div_euclid(16)..=(z + 2).div_euclid(16) {
        for cx in (x - 2).div_euclid(16)..=(x + 2).div_euclid(16) {
            chunks.push((cx, cz));
        }
    }
    chunks
}

pub fn platform_job() -> Job {
    Box::new(|sim: &mut ServerSim| -> Box<dyn Any + Send> {
        sim.create_end_platform(platform_origin());
        Box::new(())
    })
}

pub fn build_platform(level: &mut Level, player: &mut Player) -> bool {
    if !player.end.platform {
        return false;
    }
    if level.dimension != Dimension::End || player.dimension != Dimension::End {
        player.end.platform = false;
        return false;
    }
    let ready = platform_chunks()
        .into_iter()
        .all(|chunk| level.terrain.scene.generated_chunk(chunk).is_some());
    let Some(entities) = level.entities.as_mut().filter(|_| ready) else {
        return false;
    };
    entities.run_job(platform_job());
    player.end.platform = false;
    true
}

pub fn exit_dimension(spawn: &RespawnData, point: Option<&Respawn>) -> Dimension {
    respawn_dimension(spawn, point)
}

pub fn enter_the_end(player: &mut Player) {
    let from = player.dimension;
    let place = player.place.filter(|place| place.dimension == from);
    player.travel_to(
        Dimension::End,
        Some(end_arrival(place.as_ref())),
        loading_reason(false, from, Dimension::End),
    );
    player.end.platform = true;
}

pub fn leave_the_end(player: &mut Player, to: Dimension) {
    player.switch_to(to, None, loading_reason(false, Dimension::End, to));
    if let Some(switch) = player.switching.as_mut() {
        switch.respawn = true;
        switch.keep_charge = true;
    }
}

pub fn travel(globals: &Globals, level: &Level, player: &mut Player) -> Trip {
    if level.dimension != Dimension::End {
        enter_the_end(player);
        return Trip::IntoTheEnd;
    }
    let to = exit_dimension(&globals.spawn, respawn_point(player).as_ref());
    if to == Dimension::End {
        respawn_in(globals, level, player, |_| None);
    } else {
        leave_the_end(player, to);
    }
    Trip::OutOfTheEnd(to)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::commands::HOST;
    use crate::player_nbt::GlobalPos;
    use crate::survival::Survival;
    use crate::switch::LoadingReason;
    use minecraft_commands::CommandServer;
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};
    use minecraftoss_player::inventory::Inventory;

    pub(crate) fn player(dimension: Dimension) -> Player {
        let mut chat = CommandServer::new(HOST);
        chat.join(HOST, "Steve");
        let mut player = Player::new(
            chat.profile(HOST).expect("host profile"),
            PlayerMode::new(GameMode::Survival),
            Survival::default(),
            Inventory::default(),
        );
        player.dimension = dimension;
        player
    }

    #[test]
    fn the_portal_is_touched_only_between_six_and_twelve_sixteenths_of_its_block() {
        let portal = |pos| pos == (4, 30, -2);
        assert_eq!(
            touched_end_portal([4.5, 30.0, -1.5], portal),
            Some((4, 30, -2))
        );
        assert_eq!(
            touched_end_portal([4.5, 30.7, -1.5], portal),
            Some((4, 30, -2))
        );
        assert_eq!(touched_end_portal([4.5, 30.75, -1.5], portal), None);
        assert_eq!(touched_end_portal([4.5, 28.5, -1.5], portal), None);
        assert_eq!(
            touched_end_portal([4.5, 28.6, -1.5], portal),
            Some((4, 30, -2))
        );
        assert_eq!(
            touched_end_portal([3.71, 30.0, -1.5], portal),
            Some((4, 30, -2))
        );
        assert_eq!(touched_end_portal([3.7, 30.0, -1.5], portal), None);
        assert_eq!(touched_end_portal([4.5, 30.0, -1.5], |_| false), None);
    }

    #[test]
    fn the_portal_takes_a_player_at_once_then_waits_ten_ticks_after_they_leave_it() {
        use crate::portal::{DIMENSION_CHANGING_DELAY, Portal, PortalKind};
        let rules = minecraft_terrain::game_rules::GameRules::default();
        let mut portal = Portal::default();
        let mut cooldown = 0;
        let tick = |portal: &mut Portal, cooldown: &mut i32, touched: bool| {
            if touched {
                portal.set_inside(cooldown, PortalKind::End, (4, 30, -2));
            }
            portal
                .tick(cooldown, true, |kind| kind.transition_time(false, &rules))
                .is_some_and(|due| due.kind == PortalKind::End)
        };
        assert!(tick(&mut portal, &mut cooldown, true));
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY);
        for _ in 0..30 {
            assert!(
                !tick(&mut portal, &mut cooldown, true),
                "standing in it keeps the cooldown up"
            );
        }
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY - 1);
        for _ in 0..8 {
            assert!(!tick(&mut portal, &mut cooldown, false));
        }
        assert!(!tick(&mut portal, &mut cooldown, true), "one tick short");
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY - 1);
        for _ in 0..9 {
            assert!(!tick(&mut portal, &mut cooldown, false));
        }
        assert_eq!(cooldown, 0);
        assert!(
            tick(&mut portal, &mut cooldown, true),
            "the tenth tick out of it"
        );
    }

    #[test]
    fn the_end_arrival_is_on_the_platform_facing_west_keeping_pitch_and_motion() {
        let from = Place {
            motion: [0.1, -0.4, 0.2],
            on_ground: false,
            ..Place::standing(Dimension::Overworld, [812.3, 31.0, -1604.7], -35.0, 22.5)
        };
        let arrival = end_arrival(Some(&from));
        assert_eq!(arrival.dimension, Dimension::End);
        assert_eq!(arrival.feet, [100.5, 49.0, 0.5]);
        assert_eq!((arrival.yaw, arrival.pitch), (90.0, 22.5));
        assert_eq!(arrival.motion, [0.1, -0.4, 0.2]);
        assert_eq!(end_arrival(None).pitch, 0.0);
    }

    #[test]
    fn an_end_portal_in_the_overworld_or_the_nether_switches_to_the_end_platform() {
        for from in [Dimension::Overworld, Dimension::Nether] {
            let mut steve = player(from);
            steve.place = Some(Place::standing(from, [8.5, 40.0, 8.5], 180.0, -10.0));
            enter_the_end(&mut steve);
            let switch = steve.switching.expect("switching");
            assert_eq!((switch.from, switch.to), (from, Dimension::End));
            assert_eq!(switch.reason, loading_reason(false, from, Dimension::End));
            assert!(!switch.respawn);
            assert_eq!(
                steve.arrival,
                Some(end_arrival(Some(&Place::standing(
                    from,
                    [8.5, 40.0, 8.5],
                    180.0,
                    -10.0
                ))))
            );
            assert_eq!(steve.dimension, Dimension::End);
            assert!(steve.end.platform, "the platform is due on arrival");
        }
    }

    #[test]
    fn the_platform_lies_in_two_chunks_below_the_end_spawn_point() {
        assert_eq!(platform_origin(), (100, 49, 0));
        assert_eq!(platform_chunks(), [(6, -1), (6, 0)]);
    }

    #[test]
    fn the_exit_portal_goes_to_the_respawn_point_s_dimension_or_the_world_spawn_s() {
        let overworld = crate::save::world_spawn(Dimension::Overworld, (8, 64, -4));
        assert_eq!(exit_dimension(&overworld, None), Dimension::Overworld);
        let anchor = Respawn {
            at: GlobalPos {
                dimension: Dimension::Nether,
                pos: (10, 70, 10),
            },
            yaw: 0.0,
            pitch: 0.0,
            forced: true,
        };
        assert_eq!(exit_dimension(&overworld, Some(&anchor)), Dimension::Nether);
        let mut steve = player(Dimension::End);
        leave_the_end(&mut steve, Dimension::Overworld);
        let switch = steve.switching.expect("switching");
        assert_eq!(
            (switch.from, switch.to),
            (Dimension::End, Dimension::Overworld)
        );
        assert_eq!(switch.reason, LoadingReason::EndPortal);
        assert!(
            switch.respawn,
            "the respawn point is looked up in the level it arrives in"
        );
        assert_eq!(switch.arrival, None);
        assert!(!steve.dead);
    }

    fn end_level() -> Option<Level> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        let registries = std::sync::Arc::new(minecraftoss_core::Registries::load(&paths).ok()?);
        let terrain = Terrain::open(registries, 0, 2, Dimension::End, None, None).ok()?;
        let mut level = Level::new(Dimension::End, terrain);
        let data = crate::data::GameData::default();
        level.entities = Some(crate::entities::Entities::new(
            &level.terrain.stream,
            Dimension::End,
            0,
            &data,
            None,
            None,
        ));
        Some(level)
    }

    fn server_answer<T: Send + 'static>(
        level: &mut Level,
        ask: impl FnOnce(&mut ServerSim) -> T + Send + 'static,
    ) -> T {
        let (send, answer) = std::sync::mpsc::channel();
        if let Some(entities) = level.entities.as_mut() {
            entities.run_job(Box::new(
                move |sim: &mut ServerSim| -> Box<dyn Any + Send> {
                    let _ = send.send(ask(sim));
                    Box::new(())
                },
            ));
        }
        answer
            .recv_timeout(std::time::Duration::from_secs(60))
            .expect("the server thread answers")
    }

    #[test]
    fn an_arrival_rebuilds_the_platform_once_its_chunks_are_in() {
        use minecraft_terrain::scene::Block;
        use minecraft_terrain::server::PlayerEdit;
        let Some(mut level) = end_level() else {
            return;
        };
        let mut steve = player(Dimension::Overworld);
        steve.place = Some(Place::standing(
            Dimension::Overworld,
            [0.5, 70.0, 0.5],
            0.0,
            0.0,
        ));
        enter_the_end(&mut steve);
        assert!(
            !build_platform(&mut level, &mut steve),
            "no chunk is in yet"
        );
        assert!(steve.end.platform);
        let started = std::time::Instant::now();
        while platform_chunks()
            .into_iter()
            .any(|chunk| level.terrain.scene.generated_chunk(chunk).is_none())
        {
            assert!(
                started.elapsed().as_secs() < 600,
                "the platform chunks never came"
            );
            let Level {
                terrain,
                entities,
                containers,
                ..
            } = &mut level;
            crate::level::stream_tick(terrain, entities, containers, platform_origin());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        server_answer(&mut level, |sim| {
            let cobblestone = Block::new("minecraft:cobblestone");
            sim.player_edit_block((101, 50, 0), Some(&cobblestone), PlayerEdit::Place);
            sim.take_changes()
        });
        assert!(build_platform(&mut level, &mut steve));
        assert!(!steve.end.platform);
        assert!(
            !build_platform(&mut level, &mut steve),
            "one platform per arrival"
        );
        let changes = server_answer(&mut level, |sim| sim.take_changes());
        assert!(changes.contains(&((101, 50, 0), None)), "{changes:?}");
        let items = server_answer(&mut level, |sim| {
            sim.items()
                .into_iter()
                .map(|item| item.item)
                .collect::<Vec<_>>()
        });
        assert!(items.contains(&"minecraft:cobblestone".to_owned()));
        steve.end.platform = true;
        steve.dimension = Dimension::Overworld;
        assert!(!build_platform(&mut level, &mut steve));
        assert!(
            !steve.end.platform,
            "a player who never arrived drops the platform"
        );
    }
}
