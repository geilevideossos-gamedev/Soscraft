use std::collections::BTreeMap;

use bevy::math::{Mat4, Vec3};
use minecraft_net::messages::{Blob, GearView, SkatePose as NetPose, SkateReport, Skaters};
use minecraft_net::{Dimension, Lane, ToClient};
use minecraft_survival::player::Player;
use sim::voxel::BLOCK;

use crate::minecraft_players::Skating;

pub(crate) const TOP_SPEED: f32 = 40.0;
pub(crate) const STEP_SLACK: f32 = 10.0;
const LONGEST_GAP: f64 = 1.0;
pub(crate) const SILENT: f64 = 2.0;
const KEEP_ALIVE: f64 = 0.25;
const REPORT_EVERY: f64 = 0.05;
const POSE_STALE: f64 = 1.0;
const LONGEST_SPAN: f64 = 0.2;
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
    last: Option<NetPose>,
    fresh: bool,
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

    pub(crate) fn shown(&self) -> Option<NetPose> {
        self.riding.and(self.last)
    }

    pub(crate) fn take_fresh(&mut self) -> Option<NetPose> {
        std::mem::take(&mut self.fresh)
            .then_some(self.last)
            .flatten()
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
            self.last = None;
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
        self.last = report.pose.as_deref().copied();
        self.fresh = self.last.is_some();
        Verdict::Ride {
            origin: report.origin,
            yaw: report.yaw,
        }
    }

    fn refuse(&mut self, tick: u32) -> Verdict {
        self.riding = None;
        self.last = None;
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

#[derive(Clone, Debug, PartialEq)]
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
        let since = now - self.sent_at;
        if self.run.is_some() && (since < REPORT_EVERY || (!fresh && since < KEEP_ALIVE)) {
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
            pose: None,
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

fn to_3x4(m: Mat4) -> [f32; 12] {
    let mut out = [0.0; 12];
    for (i, column) in [m.x_axis, m.y_axis, m.z_axis, m.w_axis].iter().enumerate() {
        out[i * 3..i * 3 + 3].copy_from_slice(&column.truncate().to_array());
    }
    out
}

fn from_3x4(m: &[f32; 12]) -> Mat4 {
    let column = |i: usize| Vec3::new(m[i * 3], m[i * 3 + 1], m[i * 3 + 2]);
    Mat4::from_cols(
        column(0).extend(0.0),
        column(1).extend(0.0),
        column(2).extend(0.0),
        column(3).extend(1.0),
    )
}

pub(crate) fn net_pose(pose: &frame::SkatePose) -> Option<NetPose> {
    if pose.bones.len() != minecraft_net::messages::SKATE_BONES {
        return None;
    }
    Some(NetPose {
        root: to_3x4(pose.root),
        bones: std::array::from_fn(|i| to_3x4(pose.bones[i])),
    })
}

pub(crate) fn from_net(pose: &NetPose) -> frame::SkatePose {
    frame::SkatePose {
        root: from_3x4(&pose.root),
        bones: pose.bones.iter().map(from_3x4).collect(),
    }
}

fn lerp(a: Mat4, b: Mat4, t: f32) -> Mat4 {
    let (scale_a, turn_a, at_a) = a.to_scale_rotation_translation();
    let (scale_b, turn_b, at_b) = b.to_scale_rotation_translation();
    Mat4::from_scale_rotation_translation(
        scale_a.lerp(scale_b, t),
        turn_a.slerp(turn_b, t),
        at_a.lerp(at_b, t),
    )
}

fn lerp_pose(a: &frame::SkatePose, b: &frame::SkatePose, t: f32) -> frame::SkatePose {
    frame::SkatePose {
        root: lerp(a.root, b.root, t),
        bones: a
            .bones
            .iter()
            .zip(&b.bones)
            .map(|(a, b)| lerp(*a, *b, t))
            .collect(),
    }
}

#[derive(Default)]
pub(crate) struct PoseTrack {
    from: Option<(f64, frame::SkatePose)>,
    to: Option<(f64, frame::SkatePose)>,
}

impl PoseTrack {
    pub(crate) fn push(&mut self, now: f64, pose: frame::SkatePose) {
        self.from = self.to.take();
        self.to = Some((now, pose));
    }

    pub(crate) fn at(&self, now: f64) -> Option<frame::SkatePose> {
        let (heard, last) = self.to.as_ref()?;
        if now - heard > POSE_STALE {
            return None;
        }
        let Some((before, first)) = self.from.as_ref() else {
            return Some(last.clone());
        };
        let span = (heard - before).min(LONGEST_SPAN);
        if span <= 0.0 || first.bones.len() != last.bones.len() {
            return Some(last.clone());
        }
        let t = ((now - heard) / span).clamp(0.0, 1.0) as f32;
        Some(lerp_pose(first, last, t))
    }
}

#[derive(Default)]
pub(crate) struct SkaterTracks {
    tracks: BTreeMap<u32, PoseTrack>,
    gone: Vec<u32>,
}

impl SkaterTracks {
    pub(crate) fn heard(&mut self, client: u32, pose: &NetPose, now: f64) {
        self.tracks
            .entry(client)
            .or_default()
            .push(now, from_net(pose));
    }

    pub(crate) fn leave(&mut self, client: u32) {
        if self.tracks.remove(&client).is_some() {
            self.gone.push(client);
        }
    }

    pub(crate) fn heard_all(&mut self, skaters: &Skaters, own: u32, now: f64) {
        let listed = |client: &u32| skaters.0.iter().any(|(at, _)| at == client);
        let left: Vec<u32> = self
            .tracks
            .keys()
            .filter(|client| !listed(client))
            .copied()
            .collect();
        for client in left {
            self.leave(client);
        }
        for (client, pose) in skaters.0.iter().filter(|(client, _)| *client != own) {
            self.heard(*client, pose, now);
        }
    }

    pub(crate) fn show(&mut self, poses: &mut frame::SkatePoses, now: f64) {
        for client in self.gone.drain(..) {
            poses.0.remove(&client);
        }
        let mut stale = Vec::new();
        for (client, track) in &self.tracks {
            match track.at(now) {
                Some(pose) => {
                    poses.0.insert(*client, pose);
                }
                None => {
                    poses.0.remove(client);
                    stale.push(*client);
                }
            }
        }
        for client in stale {
            self.tracks.remove(&client);
        }
    }

    pub(crate) fn forget(&mut self, poses: &mut frame::SkatePoses) {
        for client in self.tracks.keys().chain(&self.gone) {
            poses.0.remove(client);
        }
        self.tracks.clear();
        self.gone.clear();
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.tracks.is_empty() && self.gone.is_empty()
    }
}

pub(crate) fn relay(skaters: &[(u32, Dimension, NetPose)], to: u32, level: Dimension) -> Skaters {
    Skaters(
        skaters
            .iter()
            .filter(|(client, dimension, _)| *client != to && *dimension == level)
            .map(|(client, _, pose)| (*client, *pose))
            .collect(),
    )
}

pub(crate) fn relayed(skaters: &[(u32, Dimension, NetPose)], to: u32, level: Dimension) -> Blob {
    let shown = relay(skaters, to, level);
    if shown.0.is_empty() {
        return Blob::default();
    }
    minecraft_net::encode(Lane::State, &shown).map_or(Blob::default(), Blob)
}

pub(crate) fn heard_skaters(blob: &Blob) -> Skaters {
    minecraft_net::decode(Lane::State, &blob.0).unwrap_or(Skaters(Vec::new()))
}

fn drawn_with(rig: bool, poses: &frame::SkatePoses, client: u32) -> Option<&frame::SkatePose> {
    poses
        .0
        .get(&client)
        .filter(|pose| rig && pose.bones.len() == frame::skate::SKATE_BONES.len())
}

pub(crate) fn drawn(poses: &frame::SkatePoses, client: u32) -> Option<&frame::SkatePose> {
    drawn_with(crate::skate::rig::reference().is_some(), poses, client)
}

pub(crate) fn board_bones<'a>(
    mode: &'a frame::SkateMode,
    client: u32,
    pose: &'a frame::SkatePose,
) -> (Vec<&'a str>, &'a [Mat4]) {
    if mode.active && mode.client == client && !mode.bones.is_empty() {
        (mode.names.iter().map(String::as_str).collect(), &mode.bones)
    } else {
        (frame::skate::SKATE_BONES.to_vec(), &pose.bones)
    }
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
            pose: None,
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

    fn turned(seed: f32) -> Mat4 {
        Mat4::from_rotation_translation(
            bevy::math::Quat::from_rotation_z(seed),
            Vec3::new(seed, -seed, seed * 3.0),
        )
    }

    fn shape(seed: f32) -> frame::SkatePose {
        frame::SkatePose {
            root: turned(seed),
            bones: (0..frame::skate::SKATE_BONES.len())
                .map(|i| turned(seed + i as f32 * 0.1))
                .collect(),
        }
    }

    fn near(a: &frame::SkatePose, b: &frame::SkatePose) -> bool {
        a.root.abs_diff_eq(b.root, 1e-4)
            && a.bones.len() == b.bones.len()
            && a.bones
                .iter()
                .zip(&b.bones)
                .all(|(a, b)| a.abs_diff_eq(*b, 1e-4))
    }

    #[test]
    fn the_shared_bones_are_the_ones_the_wire_carries() {
        assert_eq!(
            frame::skate::SKATE_BONES.len(),
            minecraft_net::messages::SKATE_BONES
        );
        let pose = shape(0.4);
        let wire = net_pose(&pose).unwrap();
        assert_eq!(
            from_net(&wire),
            pose,
            "3x4 floats keep an affine pose exactly"
        );
        let short = frame::SkatePose {
            bones: pose.bones[..5].to_vec(),
            ..pose
        };
        assert_eq!(net_pose(&short), None);
    }

    #[test]
    fn the_local_pose_keeps_only_the_mapped_bones_in_order() {
        let mut names: Vec<String> = frame::skate::SKATE_BONES
            .iter()
            .rev()
            .map(|name| (*name).to_owned())
            .collect();
        names.insert(2, "LEFTFINGER1".to_owned());
        let bones: Vec<Mat4> = (0..names.len()).map(|i| turned(i as f32)).collect();
        let mode = frame::SkateMode {
            active: true,
            root: turned(9.0),
            names: names.clone(),
            bones: bones.clone(),
            ..frame::SkateMode::default()
        };
        let pose = frame::SkatePose::of(&mode).unwrap();
        assert_eq!(pose.root, turned(9.0));
        for (i, name) in frame::skate::SKATE_BONES.iter().enumerate() {
            let at = names.iter().position(|n| n == name).unwrap();
            assert_eq!(pose.bones[i], bones[at], "{name}");
        }
        let idle = frame::SkateMode {
            active: false,
            ..mode
        };
        assert_eq!(frame::SkatePose::of(&idle), None);
    }

    #[test]
    fn the_host_relays_skaters_only_to_players_in_the_same_level() {
        let pose = |seed| net_pose(&shape(seed)).unwrap();
        let skaters = [
            (0, Dimension::Overworld, pose(0.1)),
            (2, Dimension::Overworld, pose(0.2)),
            (3, Dimension::Nether, pose(0.3)),
        ];
        let to_friend = relay(&skaters, 2, Dimension::Overworld);
        assert_eq!(
            to_friend.0,
            vec![(0, pose(0.1))],
            "the host's own skater, not the friend's own"
        );
        let to_host = relay(&skaters, 0, Dimension::Overworld);
        assert_eq!(to_host.0, vec![(2, pose(0.2))]);
        let to_nether = relay(&skaters, 4, Dimension::Nether);
        assert_eq!(to_nether.0, vec![(3, pose(0.3))]);
        assert!(relay(&skaters, 4, Dimension::End).0.is_empty());
        assert_eq!(relayed(&skaters, 4, Dimension::End), Blob::default());
        let blob = relayed(&skaters, 4, Dimension::Overworld);
        assert_eq!(blob.0.len(), 1 + 2 * (1 + 1152));
        assert_eq!(
            heard_skaters(&blob),
            relay(&skaters, 4, Dimension::Overworld)
        );
        assert_eq!(heard_skaters(&Blob::default()), Skaters::default());
    }

    #[test]
    fn others_see_a_skater_between_its_last_two_poses() {
        let mut track = PoseTrack::default();
        assert_eq!(track.at(0.0), None);
        track.push(1.0, shape(0.0));
        assert_eq!(track.at(1.02), Some(shape(0.0)));
        track.push(1.05, shape(1.0));
        assert!(
            near(&track.at(1.05).unwrap(), &shape(0.0)),
            "starts at the older pose"
        );
        let half = track.at(1.075).unwrap();
        assert!(near(&half, &lerp_pose(&shape(0.0), &shape(1.0), 0.5)));
        assert!((half.root.w_axis.x - 0.5).abs() < 1e-4);
        let turn = half.root.x_axis.y.atan2(half.root.x_axis.x);
        assert!((turn - 0.5).abs() < 1e-4, "{turn}");
        assert!(
            near(&track.at(1.2).unwrap(), &shape(1.0)),
            "holds the newest pose"
        );
        assert_eq!(
            track.at(1.05 + POSE_STALE + 0.01),
            None,
            "a silent skater is let go"
        );
    }

    #[test]
    fn a_game_shows_the_skaters_it_hears_and_drops_the_ones_that_left() {
        let mut tracks = SkaterTracks::default();
        let mut poses = frame::SkatePoses::default();
        poses.0.insert(5, shape(5.0));
        let pose = |seed| net_pose(&shape(seed)).unwrap();
        tracks.heard_all(
            &Skaters(vec![(0, pose(0.0)), (5, pose(9.0)), (7, pose(7.0))]),
            5,
            1.0,
        );
        tracks.show(&mut poses, 1.0);
        assert_eq!(poses.0.keys().copied().collect::<Vec<_>>(), vec![0, 5, 7]);
        assert_eq!(
            poses.0[&5],
            shape(5.0),
            "the game's own skater stays its own"
        );
        tracks.heard_all(&Skaters(vec![(0, pose(0.5))]), 5, 1.05);
        tracks.show(&mut poses, 1.05);
        assert_eq!(poses.0.keys().copied().collect::<Vec<_>>(), vec![0, 5]);
        tracks.show(&mut poses, 1.05 + POSE_STALE + 0.1);
        assert_eq!(poses.0.keys().copied().collect::<Vec<_>>(), vec![5]);
        tracks.heard(3, &pose(3.0), 3.0);
        tracks.show(&mut poses, 3.0);
        tracks.forget(&mut poses);
        assert_eq!(poses.0.keys().copied().collect::<Vec<_>>(), vec![5]);
        assert!(tracks.is_empty());
    }

    #[test]
    fn the_own_board_keeps_every_skate_bone_and_others_ride_the_shared_ones() {
        let pose = shape(1.0);
        let mode = frame::SkateMode {
            active: true,
            client: 4,
            names: vec!["SKATEBOARD_ROOT".to_owned(), "FRONTTRUCK".to_owned()],
            bones: vec![turned(1.0), turned(2.0)],
            ..frame::SkateMode::default()
        };
        let (names, bones) = board_bones(&mode, 4, &pose);
        assert_eq!(
            (names, bones),
            (vec!["SKATEBOARD_ROOT", "FRONTTRUCK"], &mode.bones[..])
        );
        let (names, bones) = board_bones(&mode, 2, &pose);
        assert_eq!(
            (names, bones),
            (frame::skate::SKATE_BONES.to_vec(), &pose.bones[..])
        );
    }

    #[test]
    fn a_viewer_without_the_rig_draws_the_idle_soldier() {
        let mut poses = frame::SkatePoses::default();
        poses.0.insert(1, shape(1.0));
        assert_eq!(drawn_with(true, &poses, 1), Some(&shape(1.0)));
        assert_eq!(drawn_with(false, &poses, 1), None);
        assert_eq!(drawn_with(true, &poses, 2), None);
    }

    #[test]
    fn the_host_keeps_the_pose_of_an_accepted_report_until_the_ride_ends() {
        let mut skater = skater();
        let pose = net_pose(&shape(2.0)).unwrap();
        skater.heard(
            SkateReport {
                pose: Some(Box::new(pose)),
                ..on(1, [0.0; 3])
            },
            0.0,
        );
        assert_eq!(skater.shown(), None, "nothing before the host accepts it");
        skater.judge(facts([0.0; 3], 0.0), OPEN);
        assert_eq!(skater.shown(), Some(pose));
        assert_eq!(skater.take_fresh(), Some(pose));
        assert_eq!(skater.take_fresh(), None, "taken once");
        assert_eq!(skater.shown(), Some(pose), "still relayed");
        skater.heard(off(2), 0.1);
        skater.judge(facts([0.0; 3], 0.1), OPEN);
        assert_eq!(skater.shown(), None);
        let mut far = self::skater();
        far.heard(
            SkateReport {
                pose: Some(Box::new(pose)),
                ..on(1, [blocks(STEP_SLACK + 1.0), 0.0, 0.0])
            },
            0.0,
        );
        far.judge(facts([0.0; 3], 0.0), OPEN);
        assert_eq!((far.shown(), far.take_fresh()), (None, None));
    }

    #[test]
    fn the_friend_reports_at_most_once_a_tick() {
        let mut reporter = Reporter::default();
        let at = ([0.0; 3], 0.0);
        assert!(reporter.follow(true, (1, false), at, 0.0).is_some());
        assert_eq!(reporter.follow(true, (2, false), at, 0.02), None);
        assert_eq!(reporter.follow(true, (3, false), at, 0.04), None);
        assert!(matches!(
            reporter.follow(true, (4, false), at, 0.051),
            Some(Report::State(SkateReport {
                tick: 2,
                pose: None,
                ..
            }))
        ));
    }
}
