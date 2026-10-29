use crate::game_events::{self, EventTags, Listenable, Posted, Source};
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use std::collections::{BTreeMap, HashMap};

pub type Pos = (i32, i32, i32);

pub const SENSOR: &str = "minecraft:sculk_sensor";
pub const CALIBRATED_SENSOR: &str = "minecraft:calibrated_sculk_sensor";
pub const SHRIEKER: &str = "minecraft:sculk_shrieker";
pub const CATALYST: &str = "minecraft:sculk_catalyst";
pub const LISTENER: &str = "listener";
const OCCLUSION_NUDGE: f64 = 9.999_999_747_378_752e-6;
const TRAVERSE_NUDGE: f64 = -1.0e-7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sensor,
    CalibratedSensor,
    Shrieker,
    Catalyst,
}

impl Kind {
    pub fn of(block_entity: &str) -> Option<Self> {
        Some(match block_entity {
            SENSOR => Self::Sensor,
            CALIBRATED_SENSOR => Self::CalibratedSensor,
            SHRIEKER => Self::Shrieker,
            CATALYST => Self::Catalyst,
            _ => return None,
        })
    }

    pub fn radius(self) -> i32 {
        match self {
            Self::CalibratedSensor => 16,
            Self::Sensor | Self::Shrieker | Self::Catalyst => 8,
        }
    }

    pub fn listenable(self) -> Listenable {
        match self {
            Self::Shrieker => Listenable::ShriekerCanListen,
            Self::Sensor | Self::CalibratedSensor | Self::Catalyst => Listenable::Vibrations,
        }
    }

    pub fn is_sensor(self) -> bool {
        matches!(self, Self::Sensor | Self::CalibratedSensor)
    }

    fn by_distance(self) -> bool {
        self == Self::Catalyst
    }

    fn vibrates(self) -> bool {
        self != Self::Catalyst
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Info {
    pub event: &'static str,
    pub distance: f32,
    pub pos: DVec3,
    pub source: Option<Source>,
    pub source_uuid: Option<u128>,
    pub owner: Option<Source>,
    pub owner_uuid: Option<u128>,
}

impl Info {
    fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        map.insert("game_event".to_owned(), Tag::String(self.event.to_owned()));
        map.insert("distance".to_owned(), Tag::Float(self.distance));
        map.insert(
            "pos".to_owned(),
            Tag::List(vec![
                Tag::Double(self.pos.x),
                Tag::Double(self.pos.y),
                Tag::Double(self.pos.z),
            ]),
        );
        if let Some(uuid) = self.source_uuid {
            map.insert("source".to_owned(), uuid_tag(uuid));
        }
        if let Some(uuid) = self.owner_uuid {
            map.insert("projectile_owner".to_owned(), uuid_tag(uuid));
        }
        Tag::Compound(map)
    }

    fn from_tag(tag: &Tag) -> Option<Self> {
        let event = game_events::find(tag.get("game_event")?.as_str()?)?.id;
        let distance = tag.get("distance")?.as_f64()? as f32;
        let pos = tag.get("pos")?.as_list()?;
        let axis = |i: usize| pos.get(i).and_then(Tag::as_f64);
        Some(Self {
            event,
            distance,
            pos: DVec3::new(axis(0)?, axis(1)?, axis(2)?),
            source: None,
            source_uuid: tag.get("source").and_then(tag_uuid),
            owner: None,
            owner_uuid: tag.get("projectile_owner").and_then(tag_uuid),
        })
    }
}

fn uuid_tag(uuid: u128) -> Tag {
    Tag::IntArray(minecraftoss_entities::gossip::uuid_to_ints(uuid).to_vec())
}

fn tag_uuid(tag: &Tag) -> Option<u128> {
    let ints = tag.as_ints()?;
    let [a, b, c, d] = ints[..] else {
        return None;
    };
    Some(minecraftoss_entities::gossip::uuid_from_ints([a, b, c, d]))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Selector {
    candidate: Option<(Info, i64)>,
}

impl Selector {
    pub fn add(&mut self, info: Info, tick: i64) {
        if self.replaced_by(&info, tick) {
            self.candidate = Some((info, tick));
        }
    }

    fn replaced_by(&self, info: &Info, tick: i64) -> bool {
        let Some((kept, kept_tick)) = &self.candidate else {
            return true;
        };
        if tick != *kept_tick {
            return false;
        }
        if info.distance < kept.distance {
            return true;
        }
        if info.distance > kept.distance {
            return false;
        }
        game_events::frequency(info.event) > game_events::frequency(kept.event)
    }

    pub fn chosen(&self, now: i64) -> Option<&Info> {
        self.candidate
            .as_ref()
            .filter(|(_, tick)| *tick < now)
            .map(|(info, _)| info)
    }

    pub fn candidate(&self) -> Option<&(Info, i64)> {
        self.candidate.as_ref()
    }

    fn start_over(&mut self) {
        self.candidate = None;
    }

    fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        if let Some((info, _)) = &self.candidate {
            map.insert("event".to_owned(), info.to_tag());
        }
        let tick = self.candidate.as_ref().map_or(-1, |(_, tick)| *tick);
        map.insert("tick".to_owned(), Tag::Long(tick));
        Tag::Compound(map)
    }

    fn from_tag(tag: &Tag) -> Self {
        let tick = tag.get("tick").and_then(Tag::as_i64).unwrap_or(-1);
        Self {
            candidate: tag
                .get("event")
                .and_then(Info::from_tag)
                .map(|info| (info, tick)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Listener {
    pub pos: Pos,
    pub kind: Kind,
    pub current: Option<Info>,
    pub travel: i32,
    pub selector: Selector,
    pub reload_particle: bool,
}

impl Listener {
    pub fn new(pos: Pos, kind: Kind) -> Self {
        Self {
            pos,
            kind,
            current: None,
            travel: 0,
            selector: Selector::default(),
            reload_particle: false,
        }
    }

    pub fn from_tag(pos: Pos, kind: Kind, listener: Option<&Tag>) -> Self {
        let Some(tag) = listener.filter(|tag| tag.as_compound().is_some()) else {
            return Self::new(pos, kind);
        };
        let current = tag.get("event").and_then(Info::from_tag);
        Self {
            pos,
            kind,
            reload_particle: current.is_some(),
            current,
            travel: tag
                .get("event_delay")
                .and_then(Tag::as_i64)
                .map_or(0, |delay| delay.clamp(0, i64::from(i32::MAX)) as i32),
            selector: tag
                .get("selector")
                .map(Selector::from_tag)
                .unwrap_or_default(),
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        if let Some(info) = &self.current {
            map.insert("event".to_owned(), info.to_tag());
        }
        map.insert("selector".to_owned(), self.selector.to_tag());
        map.insert("event_delay".to_owned(), Tag::Int(self.travel));
        Tag::Compound(map)
    }

    pub fn position(&self) -> DVec3 {
        centre(self.pos)
    }

    fn schedule(&mut self, posted: &Posted) {
        let info = Info {
            event: posted.event,
            distance: posted.pos.distance(self.position()) as f32,
            pos: posted.pos,
            source: posted.source,
            source_uuid: posted.source_uuid,
            owner: posted.owner,
            owner_uuid: posted.owner_uuid,
        };
        self.selector.add(info, posted.tick);
    }
}

pub trait User {
    fn can_receive(&self, listener: &Listener, at: Pos, posted: &Posted) -> bool;
    fn occludes(&self, pos: Pos) -> bool;
    fn ticks(&self, pos: Pos) -> bool;
    fn neighbours_tick(&self, pos: Pos) -> bool;
    fn receive(&mut self, listener: &Listener, info: &Info, at: Pos, distance: f32);
    fn particle(&mut self, from: DVec3, to: DVec3, ticks: i32);
    fn changed(&mut self, listener: &Listener);
    fn hear_by_distance(&mut self, _listener: &Listener, _posted: &Posted) {}
}

#[derive(Clone, Debug, Default)]
pub struct Vibrations {
    listeners: BTreeMap<Pos, Listener>,
    sections: HashMap<Pos, Vec<Pos>>,
}

impl Vibrations {
    pub fn add(&mut self, pos: Pos, kind: Kind, block_entity: Option<&Tag>) {
        if self
            .listeners
            .get(&pos)
            .is_some_and(|listener| listener.kind == kind)
        {
            return;
        }
        self.remove(pos);
        let listener =
            Listener::from_tag(pos, kind, block_entity.and_then(|tag| tag.get(LISTENER)));
        self.listeners.insert(pos, listener);
        self.sections.entry(section(pos)).or_default().push(pos);
    }

    pub fn remove(&mut self, pos: Pos) -> Option<Listener> {
        let listener = self.listeners.remove(&pos)?;
        if let Some(list) = self.sections.get_mut(&section(pos)) {
            list.retain(|&kept| kept != pos);
            if list.is_empty() {
                self.sections.remove(&section(pos));
            }
        }
        Some(listener)
    }

    pub fn remove_chunk(&mut self, x: i32, z: i32) {
        let gone: Vec<Pos> = self
            .listeners
            .keys()
            .filter(|pos| pos.0 >> 4 == x && pos.2 >> 4 == z)
            .copied()
            .collect();
        for pos in gone {
            self.remove(pos);
        }
    }

    pub fn listener(&self, pos: Pos) -> Option<&Listener> {
        self.listeners.get(&pos)
    }

    pub fn listener_mut(&mut self, pos: Pos) -> Option<&mut Listener> {
        self.listeners.get_mut(&pos)
    }

    pub fn listeners(&self) -> impl Iterator<Item = &Listener> {
        self.listeners.values()
    }

    pub fn is_empty(&self) -> bool {
        self.listeners.is_empty()
    }

    pub fn post(&mut self, tags: &EventTags, posted: &Posted, user: &mut impl User) {
        let radius = game_events::radius(posted.event);
        let at = block_of(posted.pos);
        let low = |v: i32| (v - radius) >> 4;
        let high = |v: i32| (v + radius) >> 4;
        let mut by_distance = Vec::new();
        for sx in low(at.0)..=high(at.0) {
            for sz in low(at.2)..=high(at.2) {
                for sy in low(at.1)..=high(at.1) {
                    let Some(positions) = self.sections.get(&(sx, sy, sz)) else {
                        continue;
                    };
                    for &pos in positions {
                        let Some(listener) = self.listeners.get(&pos) else {
                            continue;
                        };
                        let reach = i64::from(listener.kind.radius());
                        if distance_squared(pos, at) > reach * reach {
                            continue;
                        }
                        if listener.kind.by_distance() {
                            let distance = posted.pos.distance_squared(listener.position());
                            by_distance.push((distance, pos));
                        } else {
                            handle(&mut self.listeners, pos, tags, posted, user);
                        }
                    }
                }
            }
        }
        by_distance.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, pos) in by_distance {
            if let Some(listener) = self.listeners.get(&pos) {
                user.hear_by_distance(listener, posted);
            }
        }
    }

    pub fn force(&mut self, pos: Pos, posted: &Posted, user: &mut impl User) -> bool {
        let Some(listener) = self.listeners.get_mut(&pos) else {
            return false;
        };
        let unsourced = Posted {
            source: None,
            source_uuid: None,
            owner: None,
            owner_uuid: None,
            ..posted.clone()
        };
        if !user.can_receive(listener, pos, &unsourced) {
            return false;
        }
        listener.schedule(posted);
        true
    }

    pub fn tick(&mut self, now: i64, user: &mut impl User) {
        for listener in self.listeners.values_mut() {
            if !listener.kind.vibrates() || !user.ticks(listener.pos) {
                continue;
            }
            let mut changed = false;
            let chosen = listener
                .current
                .is_none()
                .then(|| listener.selector.chosen(now).cloned())
                .flatten();
            if let Some(info) = chosen {
                listener.travel = info.distance.floor() as i32;
                user.particle(info.pos, listener.position(), listener.travel);
                listener.current = Some(info);
                listener.selector.start_over();
                changed = true;
            }
            let Some(info) = listener.current.clone() else {
                continue;
            };
            if listener.reload_particle {
                let total = info.distance.floor() as i32;
                if total > 0 {
                    let progress = 1.0 - f64::from(listener.travel) / f64::from(total);
                    let to = listener.position();
                    user.particle(info.pos.lerp(to, progress), to, listener.travel);
                }
                listener.reload_particle = false;
            }
            listener.travel = (listener.travel - 1).max(0);
            if listener.travel <= 0
                && (!listener.kind.is_sensor() || user.neighbours_tick(listener.pos))
            {
                let at = block_of(info.pos);
                let distance = (distance_squared(at, listener.pos) as f64).sqrt() as f32;
                user.receive(listener, &info, at, distance);
                listener.current = None;
                changed = true;
            }
            if changed {
                user.changed(listener);
            }
        }
    }
}

fn handle(
    listeners: &mut BTreeMap<Pos, Listener>,
    pos: Pos,
    tags: &EventTags,
    posted: &Posted,
    user: &mut impl User,
) -> bool {
    let Some(listener) = listeners.get_mut(&pos) else {
        return false;
    };
    if listener.current.is_some() {
        return false;
    }
    if !game_events::is_valid(tags, listener.kind.listenable(), posted) {
        return false;
    }
    if !user.can_receive(listener, block_of(posted.pos), posted) {
        return false;
    }
    if occluded(posted.pos, listener.position(), |at| user.occludes(at)) {
        return false;
    }
    listener.schedule(posted);
    true
}

pub fn centre(pos: Pos) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    )
}

pub fn block_of(at: DVec3) -> Pos {
    (
        at.x.floor() as i32,
        at.y.floor() as i32,
        at.z.floor() as i32,
    )
}

fn section(pos: Pos) -> Pos {
    (pos.0 >> 4, pos.1 >> 4, pos.2 >> 4)
}

fn distance_squared(a: Pos, b: Pos) -> i64 {
    let d = |a: i32, b: i32| i64::from(a - b) * i64::from(a - b);
    d(a.0, b.0) + d(a.1, b.1) + d(a.2, b.2)
}

pub fn redstone_strength(distance: f32, radius: i32) -> i32 {
    let step = 15.0 / f64::from(radius);
    (15 - (step * f64::from(distance)).floor() as i32).max(1)
}

pub fn occluded(from: DVec3, to: DVec3, occludes: impl Fn(Pos) -> bool) -> bool {
    let centred =
        |at: DVec3| DVec3::new(at.x.floor() + 0.5, at.y.floor() + 0.5, at.z.floor() + 0.5);
    let (from, to) = (centred(from), centred(to));
    let directions = [
        DVec3::NEG_Y,
        DVec3::Y,
        DVec3::NEG_Z,
        DVec3::Z,
        DVec3::NEG_X,
        DVec3::X,
    ];
    directions
        .into_iter()
        .all(|direction| line_hits(from + direction * OCCLUSION_NUDGE, to, &occludes))
}

fn line_hits(from: DVec3, to: DVec3, test: &impl Fn(Pos) -> bool) -> bool {
    if from == to {
        return false;
    }
    let end = to.lerp(from, -TRAVERSE_NUDGE);
    let start = from.lerp(to, -TRAVERSE_NUDGE);
    let mut block = [
        start.x.floor() as i32,
        start.y.floor() as i32,
        start.z.floor() as i32,
    ];
    if test((block[0], block[1], block[2])) {
        return true;
    }
    let delta = end - start;
    let delta = [delta.x, delta.y, delta.z];
    let start = [start.x, start.y, start.z];
    let sign = |v: f64| {
        if v > 0.0 {
            1
        } else if v < 0.0 {
            -1
        } else {
            0
        }
    };
    let steps: [i32; 3] = std::array::from_fn(|axis| sign(delta[axis]));
    let t_delta: [f64; 3] = std::array::from_fn(|axis| {
        if steps[axis] == 0 {
            f64::MAX
        } else {
            f64::from(steps[axis]) / delta[axis]
        }
    });
    let frac = |v: f64| v - v.floor();
    let mut t: [f64; 3] = std::array::from_fn(|axis| {
        t_delta[axis]
            * if steps[axis] > 0 {
                1.0 - frac(start[axis])
            } else {
                frac(start[axis])
            }
    });
    while t[0] <= 1.0 || t[1] <= 1.0 || t[2] <= 1.0 {
        let axis = if t[0] < t[1] {
            if t[0] < t[2] { 0 } else { 2 }
        } else if t[1] < t[2] {
            1
        } else {
            2
        };
        block[axis] += steps[axis];
        t[axis] += t_delta[axis];
        if test((block[0], block[1], block[2])) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_events::{BLOCK_DESTROY, BLOCK_PLACE, PROJECTILE_LAND, STEP};
    use std::collections::HashSet;

    #[derive(Default)]
    struct Heard {
        walls: HashSet<Pos>,
        received: Vec<(Pos, &'static str, i32, i64)>,
        particles: Vec<(DVec3, DVec3, i32)>,
        changed: Vec<Pos>,
        now: i64,
        caught: Vec<(Pos, &'static str)>,
    }

    impl User for Heard {
        fn can_receive(&self, listener: &Listener, at: Pos, posted: &Posted) -> bool {
            let own = at == listener.pos && [BLOCK_PLACE, BLOCK_DESTROY].contains(&posted.event);
            !own && game_events::frequency(posted.event) != 0
        }

        fn occludes(&self, pos: Pos) -> bool {
            self.walls.contains(&pos)
        }

        fn ticks(&self, _: Pos) -> bool {
            true
        }

        fn neighbours_tick(&self, _: Pos) -> bool {
            true
        }

        fn receive(&mut self, listener: &Listener, info: &Info, _: Pos, distance: f32) {
            let power = redstone_strength(distance, listener.kind.radius());
            self.received
                .push((listener.pos, info.event, power, self.now));
        }

        fn particle(&mut self, from: DVec3, to: DVec3, ticks: i32) {
            self.particles.push((from, to, ticks));
        }

        fn changed(&mut self, listener: &Listener) {
            self.changed.push(listener.pos);
        }

        fn hear_by_distance(&mut self, listener: &Listener, posted: &Posted) {
            self.caught.push((listener.pos, posted.event));
        }
    }

    fn tags() -> EventTags {
        EventTags::of(
            &[
                STEP,
                BLOCK_PLACE,
                BLOCK_DESTROY,
                PROJECTILE_LAND,
                "minecraft:eat",
            ],
            &[STEP],
            &["minecraft:white_wool"],
        )
    }

    const SENSOR_AT: Pos = (8, 64, 8);

    fn sensor() -> Vibrations {
        let mut vibrations = Vibrations::default();
        vibrations.add(SENSOR_AT, Kind::Sensor, None);
        vibrations
    }

    fn walker(sneaking: bool) -> Source {
        Source::Player {
            id: 1,
            sneaking,
            spectator: false,
        }
    }

    fn run(vibrations: &mut Vibrations, heard: &mut Heard, from: i64, to: i64) {
        for now in from..=to {
            heard.now = now;
            vibrations.tick(now, heard);
        }
    }

    #[test]
    fn a_step_five_blocks_away_reaches_a_sensor_five_ticks_later_with_power_six() {
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        let feet = DVec3::new(13.5, 64.0, 8.5);
        vibrations.post(
            &tags(),
            &Posted::new(STEP, feet, 100).by(walker(false)),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 100, 104);
        assert!(heard.received.is_empty());
        assert_eq!(heard.particles.len(), 1);
        assert_eq!(heard.particles[0].1, centre(SENSOR_AT));
        assert_eq!(heard.particles[0].2, 5);
        assert!(vibrations.listener(SENSOR_AT).unwrap().current.is_some());
        run(&mut vibrations, &mut heard, 105, 106);
        assert_eq!(heard.received, vec![(SENSOR_AT, STEP, 6, 105)]);
        assert_eq!(redstone_strength(5.0, 8), 6);
        assert_eq!(redstone_strength(8.0, 8), 1);
        assert_eq!(redstone_strength(0.0, 8), 15);
        assert_eq!(heard.changed, vec![SENSOR_AT, SENSOR_AT]);
    }

    #[test]
    fn a_sneaking_step_is_not_heard_but_a_sneaking_block_place_is() {
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        let near = DVec3::new(11.5, 64.0, 8.5);
        let tags = tags();
        vibrations.post(
            &tags,
            &Posted::new(STEP, near, 10).by(walker(true)),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 11, 20);
        assert!(heard.received.is_empty());
        let placed = DVec3::new(11.5, 64.5, 8.5);
        vibrations.post(
            &tags,
            &Posted::new(BLOCK_PLACE, placed, 20).by(walker(true)),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 21, 30);
        assert_eq!(heard.received.len(), 1);
        assert_eq!(heard.received[0].1, BLOCK_PLACE);
    }

    #[test]
    fn eating_eight_blocks_away_powers_a_sensor_to_one() {
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        let tags = EventTags::of(
            &["minecraft:eat", game_events::ITEM_INTERACT_FINISH],
            &[game_events::ITEM_INTERACT_FINISH],
            &[],
        );
        let feet = DVec3::new(16.5, 64.0, 8.5);
        for event in game_events::consumed(false) {
            vibrations.post(
                &tags,
                &Posted::new(event, feet, 10).by(walker(false)),
                &mut heard,
            );
        }
        run(&mut vibrations, &mut heard, 11, 30);
        assert_eq!(heard.received.len(), 1);
        let (at, event, power, _) = heard.received[0];
        assert_eq!((at, event, power), (SENSOR_AT, "minecraft:eat", 1));
    }

    #[test]
    fn a_wool_shell_occludes_and_one_open_side_does_not() {
        let mut shell = HashSet::new();
        for dx in -1..=1 {
            for dy in -1..=1 {
                for dz in -1..=1 {
                    if (dx, dy, dz) != (0, 0, 0) {
                        shell.insert((SENSOR_AT.0 + dx, SENSOR_AT.1 + dy, SENSOR_AT.2 + dz));
                    }
                }
            }
        }
        let feet = DVec3::new(12.5, 64.0, 8.5);
        let hears = |walls: &HashSet<Pos>| {
            let mut vibrations = sensor();
            let mut heard = Heard {
                walls: walls.clone(),
                ..Heard::default()
            };
            vibrations.post(
                &tags(),
                &Posted::new(STEP, feet, 0).by(walker(false)),
                &mut heard,
            );
            run(&mut vibrations, &mut heard, 1, 10);
            !heard.received.is_empty()
        };
        assert!(hears(&HashSet::new()));
        assert!(!hears(&shell));
        let mut open = shell.clone();
        open.remove(&(SENSOR_AT.0 + 1, SENSOR_AT.1, SENSOR_AT.2));
        assert!(hears(&open), "the side facing the step is open");
        let mut far_side = shell;
        far_side.remove(&(SENSOR_AT.0 - 1, SENSOR_AT.1, SENSOR_AT.2));
        assert!(!hears(&far_side), "the open side faces away");
        let wall: HashSet<Pos> = (60..70)
            .flat_map(|y| (0..16).map(move |z| (10, y, z)))
            .collect();
        assert!(!hears(&wall), "a wall between them stops it");
        assert!(occluded(feet, centre(SENSOR_AT), |pos| wall.contains(&pos)));
        assert!(!occluded(feet, centre(SENSOR_AT), |_| false));
    }

    #[test]
    fn stepping_on_a_carpet_is_dampened() {
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        let feet = DVec3::new(10.5, 64.0625, 8.5);
        let carpet = Posted {
            dampened: true,
            ..Posted::new(STEP, feet, 0).by(walker(false))
        };
        vibrations.post(&tags(), &carpet, &mut heard);
        let wool = Source::Item {
            id: 4,
            dampens: true,
        };
        vibrations.post(
            &tags(),
            &Posted::new(PROJECTILE_LAND, feet, 0).by(wool),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 1, 10);
        assert!(heard.received.is_empty());
    }

    #[test]
    fn the_nearest_candidate_wins_and_a_tie_goes_to_the_higher_frequency() {
        let tags = tags();
        let event = |event, x: f64, tick| Posted::new(event, DVec3::new(x, 64.5, 8.5), tick);
        let winner = |posts: &[Posted]| {
            let mut vibrations = sensor();
            let mut heard = Heard::default();
            for posted in posts {
                vibrations.post(&tags, posted, &mut heard);
            }
            run(&mut vibrations, &mut heard, 11, 30);
            heard
                .received
                .first()
                .map(|(_, event, power, _)| (*event, *power))
        };
        assert_eq!(
            winner(&[event(STEP, 14.5, 10), event(BLOCK_PLACE, 11.5, 10)]),
            Some((BLOCK_PLACE, 10))
        );
        assert_eq!(
            winner(&[event(BLOCK_PLACE, 11.5, 10), event(STEP, 14.5, 10)]),
            Some((BLOCK_PLACE, 10))
        );
        assert_eq!(
            winner(&[event(BLOCK_PLACE, 12.5, 10), event(BLOCK_DESTROY, 12.5, 10)]),
            Some((BLOCK_PLACE, 8))
        );
        assert_eq!(
            winner(&[event(BLOCK_DESTROY, 12.5, 10), event(BLOCK_PLACE, 12.5, 10)]),
            Some((BLOCK_PLACE, 8))
        );
        assert_eq!(
            winner(&[event(STEP, 14.5, 9), event(BLOCK_PLACE, 11.5, 10)]),
            Some((STEP, 4)),
            "the older candidate is kept"
        );
    }

    #[test]
    fn nothing_new_is_heard_while_a_vibration_travels() {
        let tags = tags();
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        vibrations.post(
            &tags,
            &Posted::new(STEP, DVec3::new(15.5, 64.0, 8.5), 0).by(walker(false)),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 1, 3);
        vibrations.post(
            &tags,
            &Posted::new(BLOCK_PLACE, DVec3::new(9.5, 64.5, 8.5), 3),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 4, 30);
        assert_eq!(heard.received.len(), 1);
        assert_eq!(heard.received[0].1, STEP);
        let selected = vibrations.listener(SENSOR_AT).unwrap();
        assert!(selected.selector.candidate().is_none());
    }

    #[test]
    fn a_listener_with_a_travelling_vibration_round_trips_its_nbt() {
        let mut vibrations = sensor();
        let mut heard = Heard::default();
        let player = 0x0123_4567_89ab_cdef_0011_2233_4455_6677;
        let posted = Posted {
            source_uuid: Some(player),
            ..Posted::new(STEP, DVec3::new(15.5, 64.0, 8.5), 0).by(walker(false))
        };
        vibrations.post(&tags(), &posted, &mut heard);
        run(&mut vibrations, &mut heard, 1, 3);
        let travelling = vibrations.listener(SENSOR_AT).unwrap().clone();
        let tag = travelling.to_tag();
        let event = tag.get("event").expect("the travelling vibration");
        assert_eq!(event.get("game_event").and_then(Tag::as_str), Some(STEP));
        assert!(
            event
                .get("distance")
                .and_then(Tag::as_f64)
                .is_some_and(|d| (d - 7.0178344).abs() < 1e-4)
        );
        assert_eq!(
            event.get("pos").and_then(Tag::as_list).map(<[Tag]>::len),
            Some(3)
        );
        assert_eq!(event.get("source").and_then(tag_uuid), Some(player));
        assert!(event.get("projectile_owner").is_none());
        assert_eq!(tag.get("event_delay").and_then(Tag::as_i64), Some(4));
        assert_eq!(
            tag.get("selector")
                .and_then(|s| s.get("tick"))
                .and_then(Tag::as_i64),
            Some(-1)
        );
        let mut block_entity = BTreeMap::new();
        block_entity.insert(LISTENER.to_owned(), tag);
        let mut loaded = Vibrations::default();
        loaded.add(SENSOR_AT, Kind::Sensor, Some(&Tag::Compound(block_entity)));
        let back = loaded.listener(SENSOR_AT).unwrap();
        assert!(back.reload_particle);
        assert_eq!(back.travel, travelling.travel);
        let info = back.current.clone().expect("still travelling");
        assert_eq!(
            (info.event, info.pos, info.source_uuid),
            (STEP, posted.pos, Some(player))
        );
        assert_eq!(info.source, None);
        let mut heard = Heard::default();
        run(&mut loaded, &mut heard, 4, 10);
        assert_eq!(heard.received.len(), 1);
        assert_eq!(heard.received[0].3, 7);
        assert_eq!(
            heard.particles.len(),
            1,
            "the ripple is drawn again after loading"
        );
        let empty = Listener::new(SENSOR_AT, Kind::Sensor).to_tag();
        let selector = empty.get("selector").unwrap();
        assert!(selector.get("event").is_none());
        assert!(empty.get("event").is_none());
        let pending = Selector {
            candidate: Some((info, 42)),
        };
        assert_eq!(
            Selector::from_tag(&pending.to_tag())
                .candidate()
                .map(|(_, tick)| *tick),
            Some(42)
        );
    }

    #[test]
    fn a_catalyst_hears_by_distance_after_the_others() {
        let mut vibrations = Vibrations::default();
        vibrations.add((4, 64, 8), Kind::Catalyst, None);
        vibrations.add((2, 64, 8), Kind::Catalyst, None);
        vibrations.add((20, 64, 8), Kind::Catalyst, None);
        let mut heard = Heard::default();
        vibrations.post(
            &tags(),
            &Posted::new("minecraft:eat", DVec3::new(0.5, 64.5, 8.5), 0),
            &mut heard,
        );
        assert_eq!(
            heard.caught,
            vec![((2, 64, 8), "minecraft:eat"), ((4, 64, 8), "minecraft:eat")]
        );
    }

    #[test]
    fn listeners_leave_with_their_chunk_and_their_block() {
        let mut vibrations = sensor();
        vibrations.add((40, 64, 8), Kind::CalibratedSensor, None);
        vibrations.remove_chunk(0, 0);
        assert!(vibrations.listener(SENSOR_AT).is_none());
        assert!(vibrations.listener((40, 64, 8)).is_some());
        vibrations.remove((40, 64, 8));
        assert!(vibrations.is_empty());
        let mut heard = Heard::default();
        vibrations.post(
            &tags(),
            &Posted::new(STEP, centre(SENSOR_AT), 0),
            &mut heard,
        );
        run(&mut vibrations, &mut heard, 1, 20);
        assert!(heard.received.is_empty());
    }
}
