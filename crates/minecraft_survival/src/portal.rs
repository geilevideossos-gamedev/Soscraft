use minecraft_net::Dimension;
use minecraft_terrain::game_mode::GameMode;
use minecraft_terrain::game_rules::GameRules;
use minecraft_terrain::scene::Scene;
use minecraftoss_core::Registries;

use crate::level::Terrain;
use crate::player::Player;
use crate::player_nbt::Place;

pub const DIMENSION_CHANGING_DELAY: i32 = 10;
pub const TICK_SECONDS: f64 = 0.05;
pub const NETHER_PORTAL: &str = "minecraft:nether_portal";
pub const PLAYER_WIDTH: f64 = 0.6;
pub const PLAYER_HEIGHT: f64 = 1.8;
pub const BORDER_HALF_SIZE: f64 = 29_999_984.0;
pub const PORTAL_SEARCH_LIMIT: i32 = 21;
const BORDER_INSIDE: f64 = 1.0e-5;
const INSIDE_DEFLATE: f64 = 1.0e-5;
const PORTAL_THICKNESS: (f64, f64) = (6.0 / 16.0, 10.0 / 16.0);
const DECAY_PER_TICK: i32 = 4;
const CREATIVE_DELAY: &str = "players_nether_portal_creative_delay";
const DEFAULT_DELAY: &str = "players_nether_portal_default_delay";
const ENTERING_THE_NETHER: &str = "allow_entering_nether_using_portals";
const EFFECT_RISE: f32 = 0.0125;
const EFFECT_FALL: f32 = 0.05;

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
pub struct PortalEffect {
    pub intensity: f32,
    pub previous: f32,
}

impl PortalEffect {
    pub fn tick(&mut self, inside: bool) -> bool {
        self.previous = self.intensity;
        let (change, triggered) = if inside {
            (EFFECT_RISE, self.intensity == 0.0)
        } else if self.intensity > 0.0 {
            (-EFFECT_FALL, false)
        } else {
            (0.0, false)
        };
        self.intensity = (self.intensity + change).clamp(0.0, 1.0);
        triggered
    }

    pub fn shown(&self, partial: f32) -> f32 {
        self.previous + (self.intensity - self.previous) * partial.clamp(0.0, 1.0)
    }
}

pub fn overlay_alpha(intensity: f32) -> f32 {
    if intensity < 1.0 {
        let squared = intensity * intensity;
        squared * squared * 0.8 + 0.2
    } else {
        intensity
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Portal {
    pub process: Option<PortalProcess>,
    pub clock: f64,
    pub travel: Option<PortalTravel>,
    pub travel_sound: bool,
    pub effect: PortalEffect,
    pub age: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stepped {
    pub due: Option<PortalProcess>,
    pub triggered: bool,
    pub inside: bool,
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
) -> Stepped {
    let ticks = player.portal.ticks(seconds);
    let can_use = feet.is_some() && !player.dead && !player.remote;
    let invulnerable = player.mode.invulnerable();
    let mut stepped = Stepped::default();
    for _ in 0..ticks {
        let entry = feet
            .filter(|_| can_use)
            .and_then(|feet| touched_portal(feet, |pos| portal_axis_in(terrain, pos)));
        if let Some(entry) = entry {
            player
                .portal
                .set_inside(&mut player.portal_cooldown, PortalKind::Nether, entry);
        }
        let end = feet.filter(|_| can_use).and_then(|feet| {
            crate::the_end::touched_end_portal(feet, |pos| {
                crate::the_end::is_end_portal(terrain, pos)
            })
        });
        if let Some(entry) = end {
            player
                .portal
                .set_inside(&mut player.portal_cooldown, PortalKind::End, entry);
        }
        player.portal.age += 1;
        let inside = entry.is_some();
        stepped.inside |= inside;
        stepped.triggered |= player.portal.effect.tick(inside);
        stepped.due = player
            .portal
            .tick(&mut player.portal_cooldown, can_use, |kind| {
                kind.transition_time(invulnerable, rules)
            });
        if stepped.due.is_some() {
            break;
        }
    }
    stepped
}

pub fn within_border(x: f64, z: f64) -> bool {
    (-BORDER_HALF_SIZE..BORDER_HALF_SIZE).contains(&x)
        && (-BORDER_HALF_SIZE..BORDER_HALF_SIZE).contains(&z)
}

pub fn clamp_to_border(feet: [f64; 3]) -> (i32, i32, i32) {
    let clamp = |value: f64| value.clamp(-BORDER_HALF_SIZE, BORDER_HALF_SIZE - BORDER_INSIDE);
    (
        clamp(feet[0]).floor() as i32,
        feet[1].floor() as i32,
        clamp(feet[2]).floor() as i32,
    )
}

pub fn destination(feet: [f64; 3], scale: f64) -> (i32, i32, i32) {
    clamp_to_border([feet[0] * scale, feet[1], feet[2] * scale])
}

pub fn nether_portal_leads_to(from: Dimension) -> Dimension {
    if from == Dimension::Nether {
        Dimension::Overworld
    } else {
        Dimension::Nether
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub min: (i32, i32, i32),
    pub along: i32,
    pub up: i32,
}

fn shifted(pos: (i32, i32, i32), axis: PortalAxis, along: i32, up: i32) -> (i32, i32, i32) {
    match axis {
        PortalAxis::X => (pos.0 + along, pos.1 + up, pos.2),
        PortalAxis::Z => (pos.0, pos.1 + up, pos.2 + along),
    }
}

pub fn largest_rectangle_around(
    center: (i32, i32, i32),
    axis: PortalAxis,
    limit_along: i32,
    limit_up: i32,
    mut matches: impl FnMut((i32, i32, i32)) -> bool,
) -> Rect {
    let mut limit = |from: (i32, i32, i32), along: i32, up: i32, most: i32| {
        let mut count = 0;
        let mut at = from;
        while count < most {
            at = shifted(at, axis, along, up);
            if !matches(at) {
                break;
            }
            count += 1;
        }
        count
    };
    let before = limit(center, -1, 0, limit_along);
    let after = limit(center, 1, 0, limit_along);
    let mid = before as usize;
    let mut bounds = vec![(0, 0); (before + 1 + after) as usize];
    bounds[mid] = (
        limit(center, 0, -1, limit_up),
        limit(center, 0, 1, limit_up),
    );
    let base = bounds[mid].0;
    for i in 1..=before {
        let previous = bounds[mid - (i as usize - 1)];
        let column = shifted(center, axis, -i, 0);
        bounds[mid - i as usize] = (
            limit(column, 0, -1, previous.0),
            limit(column, 0, 1, previous.1),
        );
    }
    for i in 1..=after {
        let previous = bounds[mid + i as usize - 1];
        let column = shifted(center, axis, i, 0);
        bounds[mid + i as usize] = (
            limit(column, 0, -1, previous.0),
            limit(column, 0, 1, previous.1),
        );
    }
    let (mut start, mut row, mut width, mut height) = (0, 0, 0, 0);
    let mut heights = vec![0; bounds.len()];
    for at in (0..=base).rev() {
        for (column, &(below, above)) in bounds.iter().enumerate() {
            let (low, high) = (base - below, base + above);
            heights[column] = if (low..=high).contains(&at) {
                high + 1 - at
            } else {
                0
            };
        }
        let ((left, right), tall) = max_rectangle_location(&heights);
        let wide = 1 + right - left;
        if wide * tall > width * height {
            (start, row, width, height) = (left, at, wide, tall);
        }
    }
    Rect {
        min: shifted(center, axis, start - mid as i32, row - base),
        along: width,
        up: height,
    }
}

fn max_rectangle_location(heights: &[i32]) -> ((i32, i32), i32) {
    let (mut left, mut right, mut best) = (0i32, 0i32, 0i32);
    let mut stack = vec![0usize];
    for i in 1..=heights.len() {
        let current = heights.get(i).copied().unwrap_or(0);
        while let Some(&top) = stack.last() {
            let tall = heights[top];
            if current >= tall {
                stack.push(i);
                break;
            }
            stack.pop();
            let start = stack.last().map_or(0, |&below| below + 1) as i32;
            if tall * (i as i32 - start) > best * (right - left) {
                (left, right, best) = (start, i as i32, tall);
            }
        }
        if stack.is_empty() {
            stack.push(i);
        }
    }
    ((left, right - 1), best)
}

fn inverse_lerp(value: f64, from: f64, to: f64) -> f64 {
    (value - from) / (to - from)
}

pub fn relative_position(rect: Rect, axis: PortalAxis, feet: [f64; 3]) -> [f64; 3] {
    let spare_along = f64::from(rect.along) - PLAYER_WIDTH;
    let spare_up = f64::from(rect.up) - PLAYER_HEIGHT;
    let (along, across) = match axis {
        PortalAxis::X => ((feet[0], rect.min.0), (feet[2], rect.min.2)),
        PortalAxis::Z => ((feet[2], rect.min.2), (feet[0], rect.min.0)),
    };
    let x = if spare_along > 0.0 {
        inverse_lerp(
            along.0 - (f64::from(along.1) + PLAYER_WIDTH / 2.0),
            0.0,
            spare_along,
        )
        .clamp(0.0, 1.0)
    } else {
        0.5
    };
    let y = if spare_up > 0.0 {
        inverse_lerp(feet[1] - f64::from(rect.min.1), 0.0, spare_up).clamp(0.0, 1.0)
    } else {
        0.0
    };
    [x, y, across.0 - (f64::from(across.1) + 0.5)]
}

pub fn exit_position(rect: Rect, exit_axis: PortalAxis, relative: [f64; 3]) -> [f64; 3] {
    let along = PLAYER_WIDTH / 2.0 + (f64::from(rect.along) - PLAYER_WIDTH) * relative[0];
    let up = (f64::from(rect.up) - PLAYER_HEIGHT) * relative[1];
    let across = 0.5 + relative[2];
    let (x, y, z) = (
        f64::from(rect.min.0),
        f64::from(rect.min.1),
        f64::from(rect.min.2),
    );
    match exit_axis {
        PortalAxis::X => [x + along, y + up, z + across],
        PortalAxis::Z => [x + across, y + up, z + along],
    }
}

pub fn turn(entry_axis: PortalAxis, exit_axis: PortalAxis) -> f32 {
    if entry_axis == exit_axis { 0.0 } else { 90.0 }
}

pub fn turned_motion(motion: [f64; 3], turn: f32) -> [f64; 3] {
    let (sin, cos) = f64::from(-turn).to_radians().sin_cos();
    [
        motion[0] * cos + motion[2] * sin,
        motion[1],
        motion[2] * cos - motion[0] * sin,
    ]
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PortalTravel {
    pub kind: PortalKind,
    pub to: Dimension,
    pub target: (i32, i32, i32),
    pub entry_axis: PortalAxis,
    pub relative: [f64; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub motion: [f64; 3],
    pub creates: bool,
}

impl PortalTravel {
    pub fn exit(&self, rect: Rect, exit_axis: PortalAxis) -> Place {
        let turn = turn(self.entry_axis, exit_axis);
        Place {
            motion: turned_motion(self.motion, turn),
            on_ground: false,
            ..Place::standing(
                self.to,
                exit_position(rect, exit_axis, self.relative),
                self.yaw + turn,
                self.pitch,
            )
        }
    }

    pub fn target_place(&self) -> Place {
        let (x, y, z) = self.target;
        Place::standing(
            self.to,
            [f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5],
            self.yaw,
            self.pitch,
        )
    }
}

pub fn depart(
    player: &Player,
    terrain: &Terrain,
    registries: &Registries,
    due: PortalProcess,
) -> Option<PortalTravel> {
    let place = player
        .place
        .filter(|place| place.dimension == player.dimension)?;
    let to = nether_portal_leads_to(player.dimension);
    let scale = crate::switch::teleport_scale(registries, player.dimension, to).ok()?;
    let (entry_axis, relative) = match portal_axis_in(terrain, due.entry) {
        Some(axis) => {
            let rect = largest_rectangle_around(
                due.entry,
                axis,
                PORTAL_SEARCH_LIMIT,
                PORTAL_SEARCH_LIMIT,
                |pos| portal_axis_in(terrain, pos) == Some(axis),
            );
            (axis, relative_position(rect, axis, place.feet))
        }
        None => (PortalAxis::X, [0.5, 0.0, 0.0]),
    };
    Some(PortalTravel {
        kind: due.kind,
        to,
        target: destination(place.feet, scale),
        entry_axis,
        relative,
        yaw: place.yaw,
        pitch: place.pitch,
        motion: place.motion,
        creates: player.mode.mode != GameMode::Spectator,
    })
}

pub fn may_enter(rules: &GameRules, to: Dimension) -> bool {
    to != Dimension::Nether || rules.bool(ENTERING_THE_NETHER)
}

pub fn set_off(player: &mut Player, travel: PortalTravel) {
    let reason = crate::switch::loading_reason(false, player.dimension, travel.to);
    player.portal.travel = Some(travel);
    player.portal.travel_sound = true;
    player.portal.effect = PortalEffect::default();
    player.travel_to(travel.to, None, reason);
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
    fn eight_hundred_blocks_out_is_a_hundred_in_the_nether_and_back() {
        assert_eq!(destination([800.5, 70.0, -1600.5], 0.125), (100, 70, -201));
        assert_eq!(destination([800.0, 70.0, -1600.0], 0.125), (100, 70, -200));
        assert_eq!(destination([100.0, 70.9, -200.0], 8.0), (800, 70, -1600));
        assert_eq!(
            destination([-3.0e8, 64.0, 3.0e8], 8.0),
            (-29_999_984, 64, 29_999_983)
        );
        assert!(within_border(29_999_983.9, -29_999_984.0));
        assert!(!within_border(29_999_984.0, 0.0));
        assert_eq!(
            nether_portal_leads_to(Dimension::Overworld),
            Dimension::Nether
        );
        assert_eq!(
            nether_portal_leads_to(Dimension::Nether),
            Dimension::Overworld
        );
        assert_eq!(nether_portal_leads_to(Dimension::End), Dimension::Nether);
    }

    #[test]
    fn the_largest_rectangle_is_found_around_a_block_of_any_portal_shape() {
        let portal = |pos: (i32, i32, i32)| {
            pos.2 == 5 && (0..4).contains(&pos.0) && (64..69).contains(&pos.1)
        };
        assert_eq!(
            largest_rectangle_around((2, 66, 5), PortalAxis::X, 21, 21, portal),
            Rect {
                min: (0, 64, 5),
                along: 4,
                up: 5
            }
        );
        let ell = |pos: (i32, i32, i32)| {
            pos.0 == 7
                && ((0..2).contains(&pos.2) && (64..70).contains(&pos.1)
                    || (2..6).contains(&pos.2) && (64..66).contains(&pos.1))
        };
        let tall = Rect {
            min: (7, 64, 0),
            along: 2,
            up: 6,
        };
        let wide = Rect {
            min: (7, 64, 0),
            along: 6,
            up: 2,
        };
        assert_eq!(
            largest_rectangle_around((7, 66, 1), PortalAxis::Z, 21, 21, ell),
            tall
        );
        assert_eq!(
            largest_rectangle_around((7, 64, 1), PortalAxis::Z, 21, 21, ell),
            tall,
            "a tie keeps the first, taller one"
        );
        assert_eq!(
            largest_rectangle_around((7, 64, 4), PortalAxis::Z, 21, 21, ell),
            wide
        );
        assert_eq!(
            largest_rectangle_around((7, 65, 5), PortalAxis::Z, 21, 21, ell),
            wide
        );
        let lone =
            largest_rectangle_around((0, 0, 0), PortalAxis::X, 21, 21, |pos| pos == (0, 0, 0));
        assert_eq!(
            lone,
            Rect {
                min: (0, 0, 0),
                along: 1,
                up: 1
            }
        );
        let row = |pos: (i32, i32, i32)| pos.2 == 0 && pos.1 == 64;
        assert_eq!(
            largest_rectangle_around((0, 64, 0), PortalAxis::X, 21, 21, row).along,
            43
        );
        assert_eq!(max_rectangle_location(&[2, 1, 2]), ((0, 2), 1));
        assert_eq!(max_rectangle_location(&[1, 3, 3, 1]), ((1, 2), 3));
    }

    #[test]
    fn the_player_keeps_its_place_in_the_portal_and_turns_with_its_axis() {
        let entry = Rect {
            min: (0, 64, 0),
            along: 2,
            up: 3,
        };
        let relative = relative_position(entry, PortalAxis::X, [0.8, 64.6, 0.6]);
        let near = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(
            near(relative[0], 0.5 / 1.4) && near(relative[1], 0.5) && near(relative[2], 0.1),
            "{relative:?}"
        );
        assert_eq!(
            relative_position(entry, PortalAxis::X, [-5.0, 90.0, 0.5]),
            [0.0, 1.0, 0.0]
        );
        let narrow = Rect {
            min: (0, 64, 0),
            along: 0,
            up: 1,
        };
        assert_eq!(
            relative_position(narrow, PortalAxis::Z, [3.0, 64.0, 0.0]),
            [0.5, 0.0, 2.5]
        );
        let exit = Rect {
            min: (100, 70, -200),
            along: 2,
            up: 3,
        };
        let same = exit_position(exit, PortalAxis::X, relative);
        assert!(
            near(same[0], 100.8) && near(same[1], 70.6) && near(same[2], -199.4),
            "{same:?}"
        );
        let across = exit_position(exit, PortalAxis::Z, relative);
        assert!(
            near(across[0], 100.6) && near(across[1], 70.6) && near(across[2], -199.2),
            "{across:?}"
        );
        assert_eq!(turn(PortalAxis::X, PortalAxis::X), 0.0);
        assert_eq!(turn(PortalAxis::Z, PortalAxis::X), 90.0);
        let turned = turned_motion([0.2, -0.1, 0.0], 90.0);
        assert!(
            near(turned[0], 0.0) && turned[1] == -0.1 && near(turned[2], 0.2),
            "{turned:?}"
        );
        assert_eq!(turned_motion([0.2, -0.1, 0.3], 0.0), [0.2, -0.1, 0.3]);
        let travel = PortalTravel {
            kind: PortalKind::Nether,
            to: Dimension::Nether,
            target: (100, 70, -200),
            entry_axis: PortalAxis::X,
            relative,
            yaw: 30.0,
            pitch: -10.0,
            motion: [0.2, 0.0, 0.0],
            creates: true,
        };
        let place = travel.exit(exit, PortalAxis::Z);
        assert_eq!(
            (place.dimension, place.yaw, place.pitch),
            (Dimension::Nether, 120.0, -10.0)
        );
        assert_eq!(place.feet, across);
        assert!(near(place.motion[2], 0.2) && !place.on_ground);
        assert_eq!(travel.target_place().feet, [100.5, 70.0, -199.5]);
    }

    #[test]
    fn the_overlay_fades_in_over_eighty_ticks_and_out_over_twenty_and_sounds_from_zero() {
        let mut effect = PortalEffect::default();
        assert!(
            effect.tick(true),
            "stepping in from nothing triggers the sound"
        );
        assert!(!effect.tick(true));
        for _ in 2..79 {
            assert!(!effect.tick(true));
        }
        assert!(effect.intensity < 1.0);
        effect.tick(true);
        assert!(effect.intensity > 0.999, "{}", effect.intensity);
        assert!(!effect.tick(true), "no sound while it is up");
        assert_eq!(effect.intensity, 1.0);
        for left in (0..20).rev() {
            assert!(!effect.tick(false));
            assert!((effect.intensity - left as f32 * 0.05).abs() < 1e-5);
        }
        assert!(!effect.tick(false));
        assert_eq!(effect.intensity, 0.0);
        let mut short = PortalEffect::default();
        short.tick(true);
        short.tick(false);
        assert_eq!(short.intensity, 0.0);
        assert!(short.tick(true), "back to zero, the sound plays again");
        let halfway = PortalEffect {
            intensity: 0.5,
            previous: 0.4,
        };
        assert!((halfway.shown(0.5) - 0.45).abs() < 1e-6);
        assert_eq!(halfway.shown(2.0), 0.5);
        assert!((overlay_alpha(0.5) - 0.25).abs() < 1e-6);
        assert!((overlay_alpha(0.0125) - 0.2).abs() < 1e-6);
        assert_eq!(overlay_alpha(1.0), 1.0);
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
