use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::scene::Scene;

use crate::level::Terrain;
use crate::player::Player;

pub const DIMENSION_CHANGING_DELAY: i32 = 10;
pub const TICK_SECONDS: f64 = 0.05;
pub const NETHER_PORTAL: &str = "minecraft:nether_portal";
pub const PLAYER_WIDTH: f64 = 0.6;
pub const PLAYER_HEIGHT: f64 = 1.8;
const INSIDE_DEFLATE: f64 = 1.0e-5;
const PORTAL_THICKNESS: (f64, f64) = (6.0 / 16.0, 10.0 / 16.0);
const DECAY_PER_TICK: i32 = 4;
const CREATIVE_DELAY: &str = "players_nether_portal_creative_delay";
const DEFAULT_DELAY: &str = "players_nether_portal_default_delay";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalKind {
    Nether,
    End,
    Gateway,
}

impl PortalKind {
    pub fn transition_time(self, invulnerable: bool, rules: &GameRules) -> i32 {
        match self {
            Self::Nether => rules
                .int(if invulnerable {
                    CREATIVE_DELAY
                } else {
                    DEFAULT_DELAY
                })
                .max(0),
            Self::End | Self::Gateway => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortalAxis {
    X,
    Z,
}

impl PortalAxis {
    pub fn named(name: &str) -> Option<Self> {
        match name {
            "x" => Some(Self::X),
            "z" => Some(Self::Z),
            _ => None,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::X => "x",
            Self::Z => "z",
        }
    }
}

pub fn touched_portal(
    feet: [f64; 3],
    axis_at: impl Fn((i32, i32, i32)) -> Option<PortalAxis>,
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
                let Some(axis) = axis_at((x, y, z)) else {
                    continue;
                };
                let (across, base) = match axis {
                    PortalAxis::X => (2, z),
                    PortalAxis::Z => (0, x),
                };
                let from = f64::from(base) + PORTAL_THICKNESS.0;
                let to = f64::from(base) + PORTAL_THICKNESS.1;
                if low[across] < to && high[across] > from {
                    return Some((x, y, z));
                }
            }
        }
    }
    None
}

pub fn portal_axis_in(terrain: &Terrain, pos: (i32, i32, i32)) -> Option<PortalAxis> {
    let block = Scene::block(&terrain.scene, pos)?;
    if block.id.key() != NETHER_PORTAL {
        return None;
    }
    block
        .properties
        .get("axis")
        .and_then(|value| PortalAxis::named(value))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PortalProcess {
    pub kind: PortalKind,
    pub entry: (i32, i32, i32),
    pub time: i32,
    pub inside: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Portal {
    pub process: Option<PortalProcess>,
    pub clock: f64,
}

impl Portal {
    pub fn ticks(&mut self, seconds: f64) -> u32 {
        self.clock += seconds.max(0.0);
        let ticks = (self.clock / TICK_SECONDS).floor();
        self.clock -= ticks * TICK_SECONDS;
        ticks.min(f64::from(u32::MAX)) as u32
    }

    pub fn set_inside(&mut self, cooldown: &mut i32, kind: PortalKind, entry: (i32, i32, i32)) {
        if *cooldown > 0 {
            *cooldown = DIMENSION_CHANGING_DELAY;
            return;
        }
        match self.process.as_mut() {
            Some(process) if process.kind == kind => {
                if !process.inside {
                    process.entry = entry;
                    process.inside = true;
                }
            }
            _ => {
                self.process = Some(PortalProcess {
                    kind,
                    entry,
                    time: 0,
                    inside: true,
                });
            }
        }
    }

    pub fn tick(
        &mut self,
        cooldown: &mut i32,
        can_use: bool,
        transition_time: impl Fn(PortalKind) -> i32,
    ) -> Option<PortalProcess> {
        if *cooldown > 0 {
            *cooldown -= 1;
        }
        let process = self.process.as_mut()?;
        let due = if process.inside {
            process.inside = false;
            let time = process.time;
            if can_use {
                process.time += 1;
            }
            can_use && time >= transition_time(process.kind)
        } else {
            process.time = (process.time - DECAY_PER_TICK).max(0);
            false
        };
        if due {
            *cooldown = DIMENSION_CHANGING_DELAY;
            return Some(*process);
        }
        if process.time <= 0 && !process.inside {
            self.process = None;
        }
        None
    }
}

pub fn step(
    player: &mut Player,
    terrain: &Terrain,
    rules: &GameRules,
    feet: Option<[f64; 3]>,
    seconds: f64,
) -> Option<PortalProcess> {
    let ticks = player.portal.ticks(seconds);
    let can_use = feet.is_some() && !player.dead && !player.remote;
    let invulnerable = player.mode.invulnerable();
    for _ in 0..ticks {
        let entry = feet
            .filter(|_| can_use)
            .and_then(|feet| touched_portal(feet, |pos| portal_axis_in(terrain, pos)));
        if let Some(entry) = entry {
            player
                .portal
                .set_inside(&mut player.portal_cooldown, PortalKind::Nether, entry);
        }
        if let Some(due) = player
            .portal
            .tick(&mut player.portal_cooldown, can_use, |kind| {
                kind.transition_time(invulnerable, rules)
            })
        {
            return Some(due);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENTRY: (i32, i32, i32) = (10, 64, -3);

    fn stand(
        portal: &mut Portal,
        cooldown: &mut i32,
        ticks: u32,
        rules: &GameRules,
        invulnerable: bool,
    ) -> Option<(u32, PortalProcess)> {
        for tick in 1..=ticks {
            portal.set_inside(cooldown, PortalKind::Nether, ENTRY);
            if let Some(due) = portal.tick(cooldown, true, |kind| {
                kind.transition_time(invulnerable, rules)
            }) {
                return Some((tick, due));
            }
        }
        None
    }

    #[test]
    fn a_survival_player_travels_after_eighty_ticks_and_a_creative_one_at_once() {
        let rules = GameRules::default();
        assert_eq!(PortalKind::Nether.transition_time(false, &rules), 80);
        assert_eq!(PortalKind::Nether.transition_time(true, &rules), 0);
        assert_eq!(PortalKind::End.transition_time(false, &rules), 0);
        assert_eq!(PortalKind::Gateway.transition_time(false, &rules), 0);
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        let (tick, due) = stand(&mut portal, &mut cooldown, 200, &rules, false).expect("travels");
        assert_eq!(tick, 81);
        assert_eq!((due.kind, due.entry), (PortalKind::Nether, ENTRY));
        assert_eq!(cooldown, DIMENSION_CHANGING_DELAY);
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        assert_eq!(
            stand(&mut portal, &mut cooldown, 5, &rules, true).map(|(tick, _)| tick),
            Some(1)
        );
        let mut slow = GameRules::default();
        slow.set(DEFAULT_DELAY, "20").expect("an int rule");
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        assert_eq!(
            stand(&mut portal, &mut cooldown, 200, &slow, false).map(|(tick, _)| tick),
            Some(21)
        );
    }

    #[test]
    fn stepping_out_takes_four_ticks_off_a_tick_and_forgets_the_portal_at_zero() {
        let rules = GameRules::default();
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        assert!(stand(&mut portal, &mut cooldown, 41, &rules, false).is_none());
        assert_eq!(portal.process.map(|process| process.time), Some(41));
        for _ in 0..5 {
            assert!(
                portal
                    .tick(&mut cooldown, true, |kind| kind
                        .transition_time(false, &rules))
                    .is_none()
            );
        }
        assert_eq!(portal.process.map(|process| process.time), Some(21));
        let (tick, _) = stand(&mut portal, &mut cooldown, 200, &rules, false).expect("travels");
        assert_eq!(tick, 60, "the time left counts towards the next trip");
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        stand(&mut portal, &mut cooldown, 8, &rules, false);
        portal.tick(&mut cooldown, true, |_| 80);
        assert_eq!(portal.process.map(|process| process.time), Some(4));
        portal.tick(&mut cooldown, true, |_| 80);
        assert_eq!(portal.process, None);
    }

    #[test]
    fn the_cooldown_is_refreshed_inside_and_blocks_travel_until_it_runs_out() {
        let rules = GameRules::default();
        let (mut portal, mut cooldown) = (Portal::default(), DIMENSION_CHANGING_DELAY);
        for _ in 0..300 {
            portal.set_inside(&mut cooldown, PortalKind::Nether, ENTRY);
            assert!(portal.tick(&mut cooldown, true, |_| 0).is_none());
            assert_eq!(cooldown, DIMENSION_CHANGING_DELAY - 1);
        }
        assert_eq!(
            portal.process, None,
            "standing in the arrival portal never starts a trip"
        );
        for left in (0..DIMENSION_CHANGING_DELAY - 1).rev() {
            portal.tick(&mut cooldown, true, |_| 0);
            assert_eq!(cooldown, left);
        }
        let (tick, _) = stand(&mut portal, &mut cooldown, 200, &rules, true)
            .expect("travels once the cooldown ran out");
        assert_eq!(tick, 1);
    }

    #[test]
    fn a_player_who_cannot_use_portals_waits_without_counting() {
        let (mut portal, mut cooldown) = (Portal::default(), 0);
        for _ in 0..200 {
            portal.set_inside(&mut cooldown, PortalKind::Nether, ENTRY);
            assert!(portal.tick(&mut cooldown, false, |_| 0).is_none());
        }
        assert_eq!(
            portal.process, None,
            "a process that never counted has expired"
        );
        portal.set_inside(&mut cooldown, PortalKind::Nether, ENTRY);
        assert!(portal.tick(&mut cooldown, true, |_| 0).is_some());
    }

    #[test]
    fn the_entry_moves_only_on_a_tick_that_starts_outside_and_another_kind_starts_afresh() {
        let mut cooldown = 0;
        let mut portal = Portal::default();
        portal.set_inside(&mut cooldown, PortalKind::Nether, ENTRY);
        portal.set_inside(&mut cooldown, PortalKind::Nether, (11, 64, -3));
        assert_eq!(portal.process.map(|process| process.entry), Some(ENTRY));
        portal.tick(&mut cooldown, true, |_| 80);
        portal.set_inside(&mut cooldown, PortalKind::Nether, (11, 65, -3));
        assert_eq!(
            portal.process.map(|process| (process.entry, process.time)),
            Some(((11, 65, -3), 1))
        );
        portal.set_inside(&mut cooldown, PortalKind::End, (0, 50, 0));
        assert_eq!(
            portal.process.map(|process| (process.kind, process.time)),
            Some((PortalKind::End, 0))
        );
    }

    #[test]
    fn a_player_touches_a_portal_only_through_its_thin_middle() {
        let portal_x = |pos: (i32, i32, i32)| {
            (pos.0 == 0 && pos.2 == 0 && (64..67).contains(&pos.1)).then_some(PortalAxis::X)
        };
        assert_eq!(touched_portal([0.5, 64.0, 0.5], portal_x), Some((0, 64, 0)));
        assert_eq!(
            touched_portal([0.5, 64.0, 0.1], portal_x),
            Some((0, 64, 0)),
            "a box 0.3 wide reaches z 0.375"
        );
        assert_eq!(touched_portal([0.5, 64.0, 0.05], portal_x), None);
        assert_eq!(touched_portal([0.5, 64.0, 0.9], portal_x), Some((0, 64, 0)));
        assert_eq!(touched_portal([0.5, 64.0, 0.95], portal_x), None);
        assert_eq!(
            touched_portal([0.5, 62.3, 0.5], portal_x),
            Some((0, 64, 0)),
            "the head reaches into the bottom block"
        );
        assert_eq!(touched_portal([0.5, 62.1, 0.5], portal_x), None);
        let portal_z = |pos: (i32, i32, i32)| {
            (pos.0 == 0 && pos.2 == 0 && pos.1 == 64).then_some(PortalAxis::Z)
        };
        assert_eq!(
            touched_portal([0.5, 64.0, 0.05], portal_z),
            Some((0, 64, 0))
        );
        assert_eq!(touched_portal([0.05, 64.0, 0.5], portal_z), None);
        assert_eq!(PortalAxis::named("z"), Some(PortalAxis::Z));
        assert_eq!(PortalAxis::X.name(), "x");
        assert_eq!(PortalAxis::named("y"), None);
    }

    #[test]
    fn the_clock_hands_out_whole_ticks() {
        let mut portal = Portal::default();
        assert_eq!(portal.ticks(0.03), 0);
        assert_eq!(portal.ticks(0.03), 1);
        assert_eq!(portal.ticks(0.26), 5);
        assert!(portal.clock < TICK_SECONDS);
        assert_eq!(portal.ticks(-1.0), 0);
    }
}
