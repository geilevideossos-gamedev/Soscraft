use minecraft_net::messages::{GearView, SkateReport};
use minecraft_net::{Dimension, ToClient};
use minecraft_survival::player::Player;
use sim::voxel::BLOCK;

use crate::minecraft_players::Skating;

pub(crate) const TOP_SPEED: f32 = 40.0;
pub(crate) const STEP_SLACK: f32 = 10.0;
const LONGEST_GAP: f64 = 1.0;
pub(crate) const SILENT: f64 = 2.0;
const KEEP_ALIVE: f64 = 0.25;
const BODY_MINS: [f32; 3] = [-0.2 * BLOCK, -0.2 * BLOCK, 0.5 * BLOCK];
const BODY_MAXS: [f32; 3] = [0.2 * BLOCK, 0.2 * BLOCK, 1.5 * BLOCK];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Riding {
    pub(crate) airborne: bool,
    at: f64,
    teleport: u32,
    dimension: Dimension,
}

#[derive(Default)]
pub(crate) struct Skater {
    newest: Option<u32>,
    pending: Option<SkateReport>,
    riding: Option<Riding>,
    refused: bool,
    heard_at: f64,
    pub(crate) allowed: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct Facts {
    pub(crate) alive: bool,
    pub(crate) switching: bool,
    pub(crate) origin: [f32; 3],
    pub(crate) teleport: u32,
    pub(crate) dimension: Dimension,
    pub(crate) now: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Verdict {
    Idle,
    Hold,
    Ride { origin: [f32; 3], yaw: f32 },
    Stop,
    Refuse(u32),
}

impl Skater {
    pub(crate) fn heard(&mut self, report: SkateReport, now: f64) {
        if self.newest.is_some_and(|newest| report.tick <= newest) {
            return;
        }
        self.newest = Some(report.tick);
        self.heard_at = now;
        if !report.on {
            self.refused = false;
        } else if self.refused {
            return;
        }
        self.pending = Some(report);
    }

    pub(crate) fn skating(&self) -> Skating {
        self.riding.map_or_else(Skating::default, |riding| Skating {
            active: true,
            airborne: riding.airborne,
        })
    }

    pub(crate) fn judge(&mut self, facts: Facts, inside: impl Fn([f32; 3]) -> bool) -> Verdict {
        if let Some(riding) = self.riding {
            let gone = !facts.alive
                || facts.switching
                || riding.dimension != facts.dimension
                || riding.teleport != facts.teleport
                || facts.now - self.heard_at > SILENT;
            if gone {
                return self.refuse(self.newest.unwrap_or(0));
            }
        }
        let Some(report) = self.pending.take() else {
            return if self.riding.is_some() {
                Verdict::Hold
            } else {
                Verdict::Idle
            };
        };
        if !report.on {
            return if self.riding.take().is_some() {
                Verdict::Stop
            } else {
                Verdict::Idle
            };
        }
        let since = self.riding.map_or(0.0, |riding| {
            (facts.now - riding.at).clamp(0.0, LONGEST_GAP)
        });
        let reach = TOP_SPEED * since as f32 + STEP_SLACK;
        let step = distance(facts.origin, report.origin) / BLOCK;
        let sound = report.origin.iter().all(|v| v.is_finite()) && report.yaw.is_finite();
        let fine = facts.alive
            && !facts.switching
            && self.allowed
            && sound
            && step <= reach
            && !inside(report.origin);
        if !fine {
            return self.refuse(report.tick);
        }
        self.riding = Some(Riding {
            airborne: report.airborne,
            at: facts.now,
            teleport: facts.teleport,
            dimension: facts.dimension,
        });
        Verdict::Ride {
            origin: report.origin,
            yaw: report.yaw,
        }
    }

    fn refuse(&mut self, tick: u32) -> Verdict {
        self.riding = None;
        self.pending = None;
        self.refused = true;
        Verdict::Refuse(tick)
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        .sqrt()
}

pub(crate) fn ride(
    skater: &mut Skater,
    player: &Player,
    authority: &mut sim::SimWorld,
    client: sim::ClientId,
    now: f64,
) -> (Skating, Option<ToClient>) {
    let Some(ps) = authority.player(client).copied() else {
        return (Skating::default(), None);
    };
    let facts = Facts {
        alive: ps.pm_type == 0 && !player.dead,
        switching: player.switching.is_some(),
        origin: ps.origin,
        teleport: ps.e_flags & playerstate_iw4::eflags::TELEPORT,
        dimension: player.dimension,
        now,
    };
    let verdict = skater.judge(facts, |at| {
        sim::voxel::inside_blocks(at, BODY_MINS, BODY_MAXS)
    });
    let told = match verdict {
        Verdict::Ride { origin, yaw } => {
            authority.set_external_motion(client, true);
            authority.set_origin(client, origin);
            let [pitch, _, roll] = ps.viewangles;
            authority.set_viewangles(client, [pitch, yaw, roll]);
            None
        }
        Verdict::Stop => {
            authority.set_external_motion(client, false);
            None
        }
        Verdict::Refuse(tick) => {
            authority.set_external_motion(client, false);
            diag::info!(
                World,
                "Minecraft skate report {tick} of client {} refused",
                client.0
            );
            Some(ToClient::SkateRefused { tick })
        }
        Verdict::Idle | Verdict::Hold => None,
    };
    (skater.skating(), told)
}

pub(crate) fn board_view(gear: GearView, skating: bool) -> GearView {
    if skating {
        GearView {
            main: None,
            off: None,
            ..gear
        }
    } else {
        gear
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Report {
    State(SkateReport),
    Reliable(SkateReport),
}

#[derive(Default)]
pub(crate) struct Reporter {
    tick: u32,
    run: Option<u32>,
    pose: Option<u64>,
    sent_at: f64,
    stop: bool,
}

impl Reporter {
    pub(crate) fn follow(
        &mut self,
        active: bool,
        (pose, airborne): (u64, bool),
        (origin, yaw): ([f32; 3], f32),
        now: f64,
    ) -> Option<Report> {
        if !active {
            self.pose = None;
            self.run.take()?;
            return Some(Report::Reliable(self.next(false, false, origin, yaw)));
        }
        let fresh = self.pose != Some(pose);
        if !fresh && self.run.is_some() && now - self.sent_at < KEEP_ALIVE {
            return None;
        }
        self.pose = Some(pose);
        self.sent_at = now;
        let report = self.next(true, airborne, origin, yaw);
        self.run.get_or_insert(report.tick);
        Some(Report::State(report))
    }

    fn next(&mut self, on: bool, airborne: bool, origin: [f32; 3], yaw: f32) -> SkateReport {
        self.tick = self.tick.wrapping_add(1);
        SkateReport {
            tick: self.tick,
            on,
            airborne,
            origin,
            yaw,
        }
    }

    pub(crate) fn refused(&mut self, tick: u32) {
        if self.run.is_some_and(|run| tick >= run) {
            self.stop = true;
        }
    }

    pub(crate) fn take_stop(&mut self) -> bool {
        std::mem::take(&mut self.stop)
    }
}

pub(crate) fn board_yaw(root: bevy::math::Mat4) -> f32 {
    root.x_axis.y.atan2(root.x_axis.x).to_degrees()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(tick: u32, origin: [f32; 3]) -> SkateReport {
        SkateReport {
            tick,
            on: true,
            airborne: false,
            origin,
            yaw: 90.0,
        }
    }

    fn off(tick: u32) -> SkateReport {
        SkateReport {
            on: false,
            ..on(tick, [0.0; 3])
        }
    }

    fn facts(origin: [f32; 3], now: f64) -> Facts {
        Facts {
            alive: true,
            switching: false,
            origin,
            teleport: 0,
            dimension: Dimension::Overworld,
            now,
        }
    }

    fn skater() -> Skater {
        Skater {
            allowed: true,
            ..Skater::default()
        }
    }

    const OPEN: fn([f32; 3]) -> bool = |_| false;

    fn blocks(n: f32) -> f32 {
        n * BLOCK
    }

    #[test]
    fn a_valid_report_moves_the_friend() {
        let mut skater = skater();
        skater.heard(on(1, [0.0, 0.0, 2.0]), 0.0);
        let verdict = skater.judge(facts([0.0; 3], 0.0), OPEN);
        assert_eq!(
            verdict,
            Verdict::Ride {
                origin: [0.0, 0.0, 2.0],
                yaw: 90.0
            }
        );
        assert!(skater.skating().active);
        skater.heard(on(2, [blocks(5.0), 0.0, 2.0]), 0.5);
        let verdict = skater.judge(facts([0.0, 0.0, 2.0], 0.5), OPEN);
        assert!(matches!(verdict, Verdict::Ride { .. }), "{verdict:?}");
        assert_eq!(
            skater.judge(facts([blocks(5.0), 0.0, 2.0], 0.6), OPEN),
            Verdict::Hold,
            "no new report keeps the board down"
        );
        skater.heard(
            SkateReport {
                airborne: true,
                ..on(3, [blocks(6.0), 0.0, blocks(1.0)])
            },
            0.7,
        );
        skater.judge(facts([blocks(5.0), 0.0, 2.0], 0.7), OPEN);
        assert!(
            skater.skating().airborne,
            "the host's landing check sees the air"
        );
    }

    #[test]
    fn a_report_too_far_is_refused_until_the_friend_picks_the_board_up() {
        let mut skater = skater();
        skater.heard(on(1, [blocks(STEP_SLACK + 1.0), 0.0, 0.0]), 0.0);
        assert_eq!(skater.judge(facts([0.0; 3], 0.0), OPEN), Verdict::Refuse(1));
        assert!(!skater.skating().active);
        skater.heard(on(2, [0.0; 3]), 0.1);
        assert_eq!(
            skater.judge(facts([0.0; 3], 0.1), OPEN),
            Verdict::Idle,
            "nothing more is taken from that ride"
        );
        skater.heard(off(3), 0.2);
        skater.judge(facts([0.0; 3], 0.2), OPEN);
        skater.heard(on(4, [0.0; 3]), 0.3);
        assert!(matches!(
            skater.judge(facts([0.0; 3], 0.3), OPEN),
            Verdict::Ride { .. }
        ));
        let reach = TOP_SPEED * 0.5 + STEP_SLACK;
        skater.heard(on(5, [blocks(reach + 0.5), 0.0, 0.0]), 0.8);
        assert_eq!(skater.judge(facts([0.0; 3], 0.8), OPEN), Verdict::Refuse(5));
    }

    #[test]
    fn a_report_inside_a_wall_or_in_a_bad_state_is_refused() {
        let wall = |at: [f32; 3]| at[0] > blocks(2.0);
        let mut walled = skater();
        walled.heard(on(1, [blocks(3.0), 0.0, 0.0]), 0.0);
        assert_eq!(walled.judge(facts([0.0; 3], 0.0), wall), Verdict::Refuse(1));
        let mut dead = skater();
        dead.heard(on(1, [0.0; 3]), 0.0);
        let died = Facts {
            alive: false,
            ..facts([0.0; 3], 0.0)
        };
        assert_eq!(dead.judge(died, OPEN), Verdict::Refuse(1));
        let mut swimming = Skater::default();
        swimming.heard(on(1, [0.0; 3]), 0.0);
        assert_eq!(
            swimming.judge(facts([0.0; 3], 0.0), OPEN),
            Verdict::Refuse(1),
            "the board is not allowed here"
        );
        let mut broken = skater();
        broken.heard(on(1, [0.0, 0.0, f32::NAN]), 0.0);
        assert_eq!(broken.judge(facts([0.0; 3], 0.0), OPEN), Verdict::Refuse(1));
    }

    fn riding() -> Skater {
        let mut skater = skater();
        skater.heard(on(1, [0.0; 3]), 0.0);
        skater.judge(facts([0.0; 3], 0.0), OPEN);
        assert!(skater.skating().active);
        skater
    }

    #[test]
    fn death_a_dimension_change_a_teleport_or_silence_clear_the_board() {
        let dead = Facts {
            alive: false,
            ..facts([0.0; 3], 0.1)
        };
        let switching = Facts {
            switching: true,
            ..facts([0.0; 3], 0.1)
        };
        let nether = Facts {
            dimension: Dimension::Nether,
            ..facts([0.0; 3], 0.1)
        };
        let teleported = Facts {
            teleport: playerstate_iw4::eflags::TELEPORT,
            ..facts([0.0; 3], 0.1)
        };
        let silent = facts([0.0; 3], SILENT + 0.1);
        for gone in [dead, switching, nether, teleported, silent] {
            let mut skater = riding();
            assert_eq!(skater.judge(gone, OPEN), Verdict::Refuse(1), "{gone:?}");
            assert!(!skater.skating().active);
        }
    }

    #[test]
    fn an_off_report_stops_and_older_reports_are_ignored() {
        let mut skater = riding();
        skater.heard(off(3), 0.1);
        skater.heard(on(2, [0.0; 3]), 0.1);
        assert_eq!(skater.judge(facts([0.0; 3], 0.1), OPEN), Verdict::Stop);
        assert!(!skater.skating().active);
        assert_eq!(skater.judge(facts([0.0; 3], 0.2), OPEN), Verdict::Idle);
    }

    #[test]
    fn a_skater_shows_no_held_item() {
        let gear = GearView {
            main: Some(minecraft_net::messages::GearItem {
                id: "minecraft:diamond_sword".to_owned(),
                glint: false,
                dye: None,
                potion: None,
            }),
            hides_gun: true,
            ..GearView::default()
        };
        let board = board_view(gear.clone(), true);
        assert_eq!((board.main, board.off, board.hides_gun), (None, None, true));
        assert_eq!(board_view(gear.clone(), false), gear);
    }

    #[test]
    fn the_friend_reports_new_poses_keeps_the_ride_alive_and_sends_one_reliable_off() {
        let mut reporter = Reporter::default();
        let at = ([1.0, 2.0, 3.0], 45.0);
        assert_eq!(reporter.follow(false, (0, false), at, 0.0), None);
        let Some(Report::State(first)) = reporter.follow(true, (7, true), at, 0.0) else {
            panic!("the first pose is reported");
        };
        assert_eq!((first.tick, first.on, first.airborne), (1, true, true));
        assert_eq!(reporter.follow(true, (7, true), at, 0.1), None);
        assert!(matches!(
            reporter.follow(true, (7, true), at, 0.1 + KEEP_ALIVE),
            Some(Report::State(SkateReport { tick: 2, .. }))
        ));
        assert!(matches!(
            reporter.follow(true, (8, false), at, 0.4),
            Some(Report::State(SkateReport { tick: 3, .. }))
        ));
        assert!(matches!(
            reporter.follow(false, (8, false), at, 0.5),
            Some(Report::Reliable(SkateReport {
                tick: 4,
                on: false,
                ..
            }))
        ));
        assert_eq!(reporter.follow(false, (8, false), at, 0.6), None);
    }

    #[test]
    fn a_refusal_stops_only_the_ride_it_was_for() {
        let mut reporter = Reporter::default();
        let at = ([0.0; 3], 0.0);
        reporter.follow(true, (1, false), at, 0.0);
        reporter.follow(true, (2, false), at, 0.1);
        reporter.refused(2);
        assert!(reporter.take_stop());
        assert!(!reporter.take_stop());
        reporter.follow(false, (2, false), at, 0.2);
        reporter.follow(true, (3, false), at, 0.3);
        reporter.refused(2);
        assert!(
            !reporter.take_stop(),
            "an old refusal does not end a new ride"
        );
        reporter.refused(4);
        assert!(reporter.take_stop());
    }

    #[test]
    fn the_board_yaw_follows_the_skater_root() {
        let root = bevy::math::Mat4::from_rotation_z(30f32.to_radians());
        assert!((board_yaw(root) - 30.0).abs() < 1e-4);
        let root = bevy::math::Mat4::from_rotation_z(-120f32.to_radians());
        assert!((board_yaw(root) + 120.0).abs() < 1e-3);
    }
}
