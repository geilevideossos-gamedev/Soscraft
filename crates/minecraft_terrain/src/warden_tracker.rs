use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use std::collections::{BTreeMap, HashMap};

pub const TRACKER: &str = "warden_spawn_tracker";
pub const MAX_WARNING_LEVEL: i32 = 4;
pub const PLAYER_SEARCH_RADIUS: f64 = 16.0;
pub const WARNING_CHECK_DIAMETER: f64 = 48.0;
pub const DECREASE_WARNING_LEVEL_EVERY: i32 = 12000;
pub const WARNING_LEVEL_INCREASE_COOLDOWN: i32 = 200;
pub const WARDEN: &str = "minecraft:warden";
pub const DARKNESS_RADIUS: f64 = 40.0;
pub const REPLY_SPREAD: i32 = 10;
pub const REPLY_VOLUME: f32 = 5.0;
pub const REPLY_SOUNDS: [&str; 4] = [
    "minecraft:entity.warden.nearby_close",
    "minecraft:entity.warden.nearby_closer",
    "minecraft:entity.warden.nearby_closest",
    "minecraft:entity.warden.listening_angry",
];
pub const SHRIEK_DARKNESS: EffectGrant = EffectGrant {
    effect: "minecraft:darkness",
    ticks: 260,
    amplifier: 0,
    display_limit: 200,
};
const TICKS_SINCE_LAST_WARNING: &str = "ticks_since_last_warning";
const WARNING_LEVEL: &str = "warning_level";
const COOLDOWN_TICKS: &str = "cooldown_ticks";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WardenSpawnTracker {
    pub ticks_since_last_warning: i32,
    pub warning_level: i32,
    pub cooldown_ticks: i32,
}

impl WardenSpawnTracker {
    pub fn tick(&mut self) {
        if self.ticks_since_last_warning >= DECREASE_WARNING_LEVEL_EVERY {
            self.set_warning_level(self.warning_level - 1);
            self.ticks_since_last_warning = 0;
        } else {
            self.ticks_since_last_warning += 1;
        }
        if self.cooldown_ticks > 0 {
            self.cooldown_ticks -= 1;
        }
    }

    pub fn tick_by(&mut self, ticks: u32) {
        for _ in 0..ticks {
            self.tick();
        }
    }

    pub fn on_cooldown(&self) -> bool {
        self.cooldown_ticks > 0
    }

    pub fn set_warning_level(&mut self, level: i32) {
        self.warning_level = level.clamp(0, MAX_WARNING_LEVEL);
    }

    fn increase_warning_level(&mut self) {
        if !self.on_cooldown() {
            self.ticks_since_last_warning = 0;
            self.cooldown_ticks = WARNING_LEVEL_INCREASE_COOLDOWN;
            self.set_warning_level(self.warning_level + 1);
        }
    }

    pub fn to_tag(&self) -> Tag {
        let mut map = BTreeMap::new();
        for (key, value) in [
            (TICKS_SINCE_LAST_WARNING, self.ticks_since_last_warning),
            (WARNING_LEVEL, self.warning_level),
            (COOLDOWN_TICKS, self.cooldown_ticks),
        ] {
            map.insert(key.to_owned(), Tag::Int(value));
        }
        Tag::Compound(map)
    }

    pub fn from_tag(tag: &Tag) -> Self {
        let field = |key: &str| match tag.get(key) {
            None => Some(0),
            Some(value) => value
                .as_i64()
                .filter(|value| (0..=i64::from(i32::MAX)).contains(value))
                .map(|value| value as i32),
        };
        if tag.as_compound().is_none() {
            return Self::default();
        }
        match (
            field(TICKS_SINCE_LAST_WARNING),
            field(WARNING_LEVEL),
            field(COOLDOWN_TICKS),
        ) {
            (Some(ticks_since_last_warning), Some(warning_level), Some(cooldown_ticks)) => Self {
                ticks_since_last_warning,
                warning_level,
                cooldown_ticks,
            },
            _ => Self::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Warning {
    pub level: i32,
    pub players: Vec<u64>,
}

pub fn try_warn(
    centre: DVec3,
    trigger: u64,
    players: &[(u64, DVec3)],
    warden_nearby: bool,
    trackers: &mut HashMap<u64, WardenSpawnTracker>,
) -> Option<Warning> {
    if warden_nearby {
        return None;
    }
    let reach = PLAYER_SEARCH_RADIUS * PLAYER_SEARCH_RADIUS;
    let mut warned: Vec<u64> = players
        .iter()
        .filter(|(_, feet)| feet.distance_squared(centre) < reach)
        .map(|(id, _)| *id)
        .collect();
    if !warned.contains(&trigger) {
        warned.push(trigger);
    }
    let tracker = |id: &u64| {
        trackers
            .get(id)
            .map_or_else(WardenSpawnTracker::default, |tracker| *tracker)
    };
    if warned.iter().any(|id| tracker(id).on_cooldown()) {
        return None;
    }
    let highest = warned
        .iter()
        .min_by_key(|id| std::cmp::Reverse(tracker(id).warning_level))?;
    let mut raised = tracker(highest);
    raised.increase_warning_level();
    for id in &warned {
        trackers.insert(*id, raised);
    }
    Some(Warning {
        level: raised.warning_level,
        players: warned,
    })
}

pub fn warning_box(centre: DVec3) -> (DVec3, DVec3) {
    let half = DVec3::splat(WARNING_CHECK_DIAMETER / 2.0);
    (centre - half, centre + half)
}

pub fn reply_sound(warning_level: i32) -> Option<&'static str> {
    let index = usize::try_from(warning_level - 1).ok()?;
    REPLY_SOUNDS.get(index).copied()
}

pub fn players_around(centre: DVec3, radius: f64, players: &[(u64, DVec3)]) -> Vec<u64> {
    players
        .iter()
        .filter(|(_, feet)| feet.distance_squared(centre) < radius * radius)
        .map(|(id, _)| *id)
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectGrant {
    pub effect: &'static str,
    pub ticks: u32,
    pub amplifier: u8,
    pub display_limit: u32,
}

impl EffectGrant {
    pub fn takes(&self, survival_like: bool, held: Option<(u32, u8)>) -> bool {
        let ends_within =
            |ticks: u32| ticks != u32::MAX && ticks <= self.display_limit.saturating_sub(1);
        survival_like
            && held
                .is_none_or(|(ticks, amplifier)| amplifier < self.amplifier || ends_within(ticks))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHRIEKER: DVec3 = DVec3::new(0.5, 64.5, 0.5);

    fn feet(x: f64) -> DVec3 {
        DVec3::new(x, 64.0, 0.5)
    }

    fn ticked(tracker: &mut WardenSpawnTracker, ticks: i32) {
        for _ in 0..ticks {
            tracker.tick();
        }
    }

    #[test]
    fn each_shriek_raises_the_level_with_a_ten_second_cooldown() {
        let mut trackers = HashMap::new();
        let players = [(1, feet(3.5))];
        for level in 1..=4 {
            let warning = try_warn(SHRIEKER, 1, &players, false, &mut trackers);
            assert_eq!(warning.map(|w| w.level), Some(level));
            assert_eq!(trackers[&1].cooldown_ticks, 200);
            assert_eq!(trackers[&1].ticks_since_last_warning, 0);
            let tracker = trackers.get_mut(&1).unwrap();
            ticked(tracker, 199);
            assert!(tracker.on_cooldown());
            assert_eq!(try_warn(SHRIEKER, 1, &players, false, &mut trackers), None);
            ticked(trackers.get_mut(&1).unwrap(), 1);
            assert!(!trackers[&1].on_cooldown());
        }
        assert_eq!(
            try_warn(SHRIEKER, 1, &players, false, &mut trackers).map(|w| w.level),
            Some(4),
            "four is the most"
        );
    }

    #[test]
    fn players_within_sixteen_rise_together_and_one_at_twenty_does_not() {
        let mut trackers = HashMap::new();
        trackers.insert(
            2,
            WardenSpawnTracker {
                warning_level: 2,
                ..WardenSpawnTracker::default()
            },
        );
        let players = [(1, feet(1.5)), (2, feet(15.0)), (3, feet(20.5))];
        let warning = try_warn(SHRIEKER, 1, &players, false, &mut trackers).unwrap();
        assert_eq!(warning.level, 3, "the highest of them rises");
        assert_eq!(warning.players, vec![1, 2]);
        assert_eq!(trackers[&1], trackers[&2]);
        assert_eq!(trackers[&1].warning_level, 3);
        assert!(!trackers.contains_key(&3), "twenty blocks away");
        let far = try_warn(SHRIEKER, 3, &[(3, feet(20.5))], false, &mut trackers).unwrap();
        assert_eq!(
            far.players,
            vec![3],
            "a trigger player out of reach still counts"
        );
        assert_eq!(far.level, 1);
    }

    #[test]
    fn a_player_on_cooldown_blocks_everyone() {
        let mut trackers = HashMap::new();
        trackers.insert(
            2,
            WardenSpawnTracker {
                warning_level: 1,
                cooldown_ticks: 5,
                ..WardenSpawnTracker::default()
            },
        );
        let players = [(1, feet(2.5)), (2, feet(4.5))];
        assert_eq!(try_warn(SHRIEKER, 1, &players, false, &mut trackers), None);
        assert!(!trackers.contains_key(&1), "nobody changes");
        assert_eq!(trackers[&2].warning_level, 1);
    }

    #[test]
    fn a_warden_within_the_box_stops_the_warning() {
        let mut trackers = HashMap::new();
        let players = [(1, feet(2.5))];
        assert_eq!(try_warn(SHRIEKER, 1, &players, true, &mut trackers), None);
        assert!(trackers.is_empty());
        let (low, high) = warning_box(SHRIEKER);
        assert_eq!(low, DVec3::new(-23.5, 40.5, -23.5));
        assert_eq!(high, DVec3::new(24.5, 88.5, 24.5));
    }

    #[test]
    fn the_level_drops_one_every_ten_minutes() {
        let mut tracker = WardenSpawnTracker {
            warning_level: 3,
            ..WardenSpawnTracker::default()
        };
        ticked(&mut tracker, 12000);
        assert_eq!(tracker.warning_level, 3);
        assert_eq!(tracker.ticks_since_last_warning, 12000);
        ticked(&mut tracker, 1);
        assert_eq!(tracker.warning_level, 2);
        assert_eq!(tracker.ticks_since_last_warning, 0);
        ticked(&mut tracker, 12001 * 3);
        assert_eq!(tracker.warning_level, 0, "it never goes below zero");
    }

    #[test]
    fn it_round_trips_in_the_player_file() {
        let tracker = WardenSpawnTracker {
            ticks_since_last_warning: 345,
            warning_level: 2,
            cooldown_ticks: 17,
        };
        let tag = tracker.to_tag();
        assert_eq!(
            tag.get("ticks_since_last_warning").and_then(Tag::as_i64),
            Some(345)
        );
        assert_eq!(tag.get("warning_level").and_then(Tag::as_i64), Some(2));
        assert_eq!(tag.get("cooldown_ticks").and_then(Tag::as_i64), Some(17));
        assert_eq!(WardenSpawnTracker::from_tag(&tag), tracker);
        let mut partial = BTreeMap::new();
        partial.insert("warning_level".to_owned(), Tag::Int(3));
        assert_eq!(
            WardenSpawnTracker::from_tag(&Tag::Compound(partial)),
            WardenSpawnTracker {
                warning_level: 3,
                ..WardenSpawnTracker::default()
            }
        );
        let mut negative = BTreeMap::new();
        negative.insert("warning_level".to_owned(), Tag::Int(2));
        negative.insert("cooldown_ticks".to_owned(), Tag::Int(-1));
        assert_eq!(
            WardenSpawnTracker::from_tag(&Tag::Compound(negative)),
            WardenSpawnTracker::default(),
            "a broken tracker starts over"
        );
    }

    #[test]
    fn the_replies_grow_closer_and_angrier() {
        assert_eq!(reply_sound(0), None);
        assert_eq!(reply_sound(1), Some("minecraft:entity.warden.nearby_close"));
        assert_eq!(
            reply_sound(3),
            Some("minecraft:entity.warden.nearby_closest")
        );
        assert_eq!(
            reply_sound(4),
            Some("minecraft:entity.warden.listening_angry")
        );
        assert_eq!(reply_sound(5), None);
        let players = [(1, feet(39.0)), (2, feet(41.0))];
        assert_eq!(players_around(SHRIEKER, DARKNESS_RADIUS, &players), vec![1]);
    }

    #[test]
    fn player_effects_follow_vanilla_s_rules() {
        let grant = SHRIEK_DARKNESS;
        assert!(grant.takes(true, None), "survival, no darkness yet");
        assert!(!grant.takes(false, None), "creative or spectator");
        assert!(
            !grant.takes(true, Some((240, 0))),
            "a long darkness is kept"
        );
        assert!(
            grant.takes(true, Some((199, 0))),
            "one ending within 199 ticks is renewed"
        );
        assert!(!grant.takes(true, Some((200, 0))));
        assert!(
            !grant.takes(true, Some((u32::MAX, 0))),
            "an endless one is kept"
        );
        assert!(!grant.takes(true, Some((240, 1))), "a stronger one is kept");
        let stronger = EffectGrant {
            amplifier: 1,
            ..grant
        };
        assert!(
            stronger.takes(true, Some((240, 0))),
            "a weaker one is replaced"
        );
    }
}
