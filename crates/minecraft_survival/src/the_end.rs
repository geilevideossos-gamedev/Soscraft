use minecraft_net::Dimension;
use minecraft_terrain::level_storage::RespawnData;
use minecraft_terrain::scene::Scene;

use crate::level::{Level, Terrain};
use crate::player::Player;
use crate::player_nbt::{Place, Respawn};
use crate::respawn::{respawn_dimension, respawn_in, respawn_point};
use crate::server::Globals;
use crate::switch::loading_reason;

pub const END_PORTAL: &str = "minecraft:end_portal";
pub const END_SPAWN_POINT: (i32, i32, i32) = (100, 50, 0);
pub const DIMENSION_CHANGING_DELAY: i32 = 10;
pub const TICK_SECONDS: f64 = 0.05;
pub const WEST_YAW: f32 = 90.0;
const PLAYER_WIDTH: f64 = 0.6;
const PLAYER_HEIGHT: f64 = 1.8;
const INSIDE_DEFLATE: f64 = 1.0e-5;
const PORTAL_SHAPE: (f64, f64) = (6.0 / 16.0, 12.0 / 16.0);

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EndTravel {
    pub clock: f64,
}

impl EndTravel {
    pub fn ticks(&mut self, seconds: f64) -> u32 {
        self.clock += seconds.max(0.0);
        let ticks = (self.clock / TICK_SECONDS).floor();
        self.clock -= ticks * TICK_SECONDS;
        ticks.min(f64::from(u32::MAX)) as u32
    }
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

pub fn portal_tick(cooldown: &mut i32, touched: bool) -> bool {
    if *cooldown > 0 {
        *cooldown -= 1;
    }
    if !touched {
        return false;
    }
    let due = *cooldown == 0;
    *cooldown = DIMENSION_CHANGING_DELAY;
    due
}

pub fn step(player: &mut Player, terrain: &Terrain, feet: Option<[f64; 3]>, seconds: f64) -> bool {
    let ticks = player.end.ticks(seconds);
    let can_use = feet.is_some() && !player.dead && !player.remote;
    for _ in 0..ticks {
        let touched = feet
            .filter(|_| can_use)
            .and_then(|feet| touched_end_portal(feet, |pos| is_end_portal(terrain, pos)));
        if portal_tick(&mut player.portal_cooldown, touched.is_some()) {
            return true;
        }
    }
    false
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
}

pub fn leave_the_end(player: &mut Player, to: Dimension) {
    player.switch_to(to, None, loading_reason(false, Dimension::End, to));
    if let Some(switch) = player.switching.as_mut() {
        switch.respawn = true;
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
mod tests {
    use super::*;
    use crate::commands::HOST;
    use crate::player_nbt::GlobalPos;
    use crate::survival::Survival;
    use crate::switch::LoadingReason;
    use minecraft_commands::CommandServer;
    use minecraft_terrain::game_mode::{GameMode, PlayerMode};
    use minecraftoss_player::inventory::Inventory;

    fn player(dimension: Dimension) -> Player {
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
        let mut cooldown = 0;
        assert!(portal_tick(&mut cooldown, true));
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY);
        for _ in 0..30 {
            assert!(
                !portal_tick(&mut cooldown, true),
                "standing in it keeps the cooldown up"
            );
        }
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY);
        for _ in 0..8 {
            assert!(!portal_tick(&mut cooldown, false));
        }
        assert!(!portal_tick(&mut cooldown, true), "one tick short");
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY);
        for _ in 0..9 {
            assert!(!portal_tick(&mut cooldown, false));
        }
        assert_eq!(cooldown, 1);
        assert!(portal_tick(&mut cooldown, true), "the tenth tick out of it");
    }

    #[test]
    fn the_travel_clock_counts_whole_ticks() {
        let mut travel = EndTravel::default();
        assert_eq!(travel.ticks(0.03), 0);
        assert_eq!(travel.ticks(0.03), 1);
        assert_eq!(travel.ticks(0.26), 5);
        assert_eq!(travel.ticks(-1.0), 0);
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
        }
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
}
