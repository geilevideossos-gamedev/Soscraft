use crate::ambient_particles::{AmbientParticles, Kind};
use crate::spell_particles::{SpellKind, SpellParticles};
use elden_boss::data::{BossDef, Burst, Cue, CueKey, Shape};
use elden_boss::shape::Frame;
use elden_boss::{Clip, State};
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;

pub const GOLD: [f32; 3] = [1.0, 0.82, 0.25];
pub const CATCH_UP: u16 = 4;
pub const MIST_TRIES: usize = 32;
pub const MIST_SPELLS: usize = 16;
pub const SPARKS: usize = 10;
pub const SPARK_SPEED: f64 = 0.06;
pub const SLAM_POOFS: usize = 16;
pub const SLAM_RADIUS: f64 = 1.0;
pub const SLAM_SPEED: f64 = 0.25;
pub const SLAM_AHEAD: f64 = 1.5;
pub const CRITS: usize = 16;
pub const CHEST: f64 = 0.6;
pub const SHIMMER: usize = 2;

const STATE_BITS: u32 = 4;
const INDEX_BITS: u32 = 6;
const CLOCK_BITS: u32 = 12;
const SEEN: u32 = 1 << 30;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memory {
    pub state: u32,
    pub index: u32,
    pub clock: u16,
    pub criticals: u8,
}

fn code(state: State) -> u32 {
    State::ALL
        .iter()
        .position(|known| *known == state)
        .map_or(0, |at| at as u32)
}

pub fn remember(clip: &Clip) -> i32 {
    let index = u32::from(clip.index) & ((1 << INDEX_BITS) - 1);
    let clock = u32::from(clip.clock.min((1 << CLOCK_BITS) - 1));
    let packed = code(clip.state)
        | index << STATE_BITS
        | clock << (STATE_BITS + INDEX_BITS)
        | u32::from(clip.criticals) << (STATE_BITS + INDEX_BITS + CLOCK_BITS)
        | SEEN;
    packed as i32
}

pub fn recall(memory: i32) -> Option<Memory> {
    let packed = memory as u32;
    if packed & SEEN == 0 {
        return None;
    }
    let field = |shift: u32, bits: u32| (packed >> shift) & ((1 << bits) - 1);
    Some(Memory {
        state: field(0, STATE_BITS),
        index: field(STATE_BITS, INDEX_BITS),
        clock: field(STATE_BITS + INDEX_BITS, CLOCK_BITS) as u16,
        criticals: field(STATE_BITS + INDEX_BITS + CLOCK_BITS, 8) as u8,
    })
}

pub fn cue_table(def: &'static BossDef, clip: &Clip) -> &'static [CueKey] {
    let clips = &def.clips;
    match clip.state {
        State::Move => clip.played(def).map_or(&[], |mv| mv.cues),
        State::Wake => clips.wake.cues,
        State::Kneel => clips.kneel.cues,
        State::Fade => clips.fade.cues,
        State::Dead => clips.death.cues,
        State::Shift => def.shift(clip.phase).map_or(&[], |shift| shift.cues),
        _ => &[],
    }
}

pub fn bursts(def: &'static BossDef, clip: &Clip, last: Option<i32>) -> Vec<(u16, Burst)> {
    let last = last.and_then(recall);
    let index = u32::from(clip.index) & ((1 << INDEX_BITS) - 1);
    let same = last.filter(|last| {
        last.state == code(clip.state) && last.index == index && last.clock <= clip.clock
    });
    let from = match same {
        Some(last) => last.clock.saturating_add(1),
        None => 0,
    };
    let from = from.max(clip.clock.saturating_sub(CATCH_UP));
    let mut out: Vec<(u16, Burst)> = cue_table(def, clip)
        .iter()
        .filter(|key| from <= key.at && key.at <= clip.clock)
        .filter_map(|key| match key.cue {
            Cue::Burst(burst) => Some((key.at, burst)),
            Cue::Sound(_) => None,
        })
        .collect();
    let critical = last.is_some_and(|last| last.criticals != clip.criticals);
    if critical && clip.state == State::Kneel {
        out.extend(def.cues.critical.iter().filter_map(|cue| match cue {
            Cue::Burst(burst) => Some((clip.clock, *burst)),
            Cue::Sound(_) => None,
        }));
    }
    out
}

pub fn slam_centre(def: &'static BossDef, clip: &Clip, at: u16, frame: &Frame) -> DVec3 {
    let strikes = clip.played(def).map_or(&[][..], |mv| mv.strikes);
    let landing = strikes
        .iter()
        .filter(|strike| strike.from <= at && at <= strike.to)
        .map(|strike| match strike.shape {
            Shape::Circle { ahead, .. } | Shape::Ring { ahead, .. } => (0, ahead),
            Shape::Arc { reach, .. } => (1, reach / 2.0),
        })
        .min_by(|a, b| a.0.cmp(&b.0));
    frame.ahead(landing.map_or(SLAM_AHEAD, |(_, ahead)| ahead))
}

fn unit(random: &mut LegacyRandom) -> f64 {
    random.next_double() * 2.0 - 1.0
}

fn within(def: &BossDef, feet: DVec3, random: &mut LegacyRandom) -> DVec3 {
    let half = def.width / 2.0;
    feet + DVec3::new(
        unit(random) * half,
        random.next_double() * def.height,
        unit(random) * half,
    )
}

pub struct Sparks<'a> {
    pub ambient: &'a mut AmbientParticles,
    pub spell: &'a mut SpellParticles,
    pub random: &'a mut LegacyRandom,
}

impl Sparks<'_> {
    pub fn mist(&mut self, def: &BossDef, feet: DVec3) {
        for _ in 0..MIST_TRIES {
            let (x, y, z) = (unit(self.random), unit(self.random), unit(self.random));
            if x * x + y * y + z * z > 1.0 {
                continue;
            }
            let at = feet
                + DVec3::new(
                    def.width * x / 2.0,
                    def.height * (0.5 + y / 2.0),
                    def.width * z / 2.0,
                );
            self.ambient
                .add(Kind::Totem, at, DVec3::new(x, y + 0.2, z) * 0.6);
        }
        for _ in 0..MIST_SPELLS {
            let at = within(def, feet, self.random);
            self.spell
                .spawn(SpellKind::Effect, at, DVec3::ZERO, GOLD, 1.0);
        }
    }

    pub fn sparks(&mut self, def: &BossDef, frame: &Frame) {
        let hand = frame.at(def.daggers.hand);
        for _ in 0..SPARKS {
            let motion =
                DVec3::new(unit(self.random), unit(self.random), unit(self.random)) * SPARK_SPEED;
            self.ambient.add(Kind::EndRod, hand, motion);
        }
    }

    pub fn slam(&mut self, centre: DVec3) {
        for step in 0..SLAM_POOFS {
            let angle = std::f64::consts::TAU * step as f64 / SLAM_POOFS as f64;
            let out = DVec3::new(angle.cos(), 0.0, angle.sin());
            let at = centre + out * SLAM_RADIUS + DVec3::Y * 0.1;
            self.ambient
                .add(Kind::Poof, at, out * SLAM_SPEED + DVec3::Y * 0.05);
        }
        self.ambient
            .add(Kind::Explosion, centre + DVec3::Y * 0.5, DVec3::X);
    }

    pub fn crits(&mut self, def: &BossDef, feet: DVec3) {
        let chest = feet + DVec3::Y * (def.height * CHEST);
        for _ in 0..CRITS {
            let motion = DVec3::new(
                unit(self.random),
                unit(self.random) * 0.5 + 0.5,
                unit(self.random),
            );
            self.ambient.add(Kind::Crit, chest, motion);
        }
    }

    pub fn shimmer(&mut self, def: &BossDef, feet: DVec3) {
        for _ in 0..SHIMMER {
            let at = within(def, feet, self.random);
            self.spell
                .spawn(SpellKind::Effect, at, DVec3::ZERO, GOLD, 1.0);
        }
    }

    pub fn burst(
        &mut self,
        def: &'static BossDef,
        clip: &Clip,
        (at, burst): (u16, Burst),
        frame: &Frame,
    ) {
        match burst {
            Burst::GoldenMist => self.mist(def, frame.origin),
            Burst::Sparks => self.sparks(def, frame),
            Burst::Slam => self.slam(slam_centre(def, clip, at, frame)),
            Burst::Crit => self.crits(def, frame.origin),
        }
    }

    pub fn show(
        &mut self,
        def: &'static BossDef,
        clip: &Clip,
        frame: &Frame,
        last: Option<i32>,
    ) -> i32 {
        for burst in bursts(def, clip, last) {
            self.burst(def, clip, burst, frame);
        }
        if clip.awake && clip.phase >= 2 && clip.state != State::Dead {
            self.shimmer(def, frame.origin);
        }
        remember(clip)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elden_boss::margit::MARGIT;

    fn shown_at(state: State, key: &str, clock: u16) -> Clip {
        Clip {
            state,
            index: MARGIT.move_index(key).map_or(0, |index| index as u8),
            clock,
            held: false,
            phase: 1,
            awake: true,
            from: 0,
            criticals: 0,
        }
    }

    fn fired(clip: &Clip, last: Option<&Clip>) -> Vec<(u16, Burst)> {
        bursts(&MARGIT, clip, last.map(remember))
    }

    #[test]
    fn what_he_showed_last_is_remembered_in_one_number() {
        for state in State::ALL {
            let clip = Clip {
                index: 7,
                clock: 81,
                criticals: 3,
                ..shown_at(state, "", 0)
            };
            let memory = recall(remember(&clip)).unwrap();
            assert_eq!(
                (memory.state, memory.index, memory.clock, memory.criticals),
                (code(state), 7, 81, 3)
            );
        }
        assert_eq!(recall(0), None, "nothing seen yet");
    }

    #[test]
    fn each_burst_fires_once_on_its_tick_and_skipped_ticks_catch_up() {
        let slam = |clock: u16| shown_at(State::Move, "cane_slam", clock);
        assert!(fired(&slam(28), Some(&slam(27))).is_empty());
        assert_eq!(fired(&slam(29), Some(&slam(28))), [(29, Burst::Slam)]);
        assert!(fired(&slam(29), Some(&slam(29))).is_empty(), "not twice");
        assert_eq!(fired(&slam(31), Some(&slam(27))), [(29, Burst::Slam)]);
        assert_eq!(fired(&slam(29), None), [(29, Burst::Slam)]);
        assert!(
            fired(&slam(40), None).is_empty(),
            "a late look sees no old slam"
        );
        let wake = shown_at(State::Wake, "", 0);
        let asleep = shown_at(State::Asleep, "", 7);
        assert_eq!(fired(&wake, Some(&asleep)), [(0, Burst::GoldenMist)]);
        let sweep = shown_at(State::Move, "sword_sweep", 0);
        let combo = shown_at(State::Move, "cane_combo", 52);
        assert_eq!(
            fired(&sweep, Some(&combo)),
            [(0, Burst::Sparks)],
            "a branch starts a clip"
        );
        let kneel = shown_at(State::Kneel, "", 30);
        let critical = Clip {
            clock: 60,
            criticals: 1,
            ..kneel
        };
        assert_eq!(fired(&critical, Some(&kneel)), [(60, Burst::Crit)]);
        let reset = Clip {
            state: State::Return,
            ..critical
        };
        assert!(fired(&reset, Some(&kneel)).is_empty());
    }

    #[test]
    fn a_slam_rings_dust_where_his_strike_lands() {
        let frame = Frame {
            origin: DVec3::new(10.0, 64.0, 10.0),
            yaw: 0.0,
        };
        let leap = shown_at(State::Move, "leap_slam", 30);
        assert_eq!(
            slam_centre(&MARGIT, &leap, 30, &frame),
            DVec3::new(10.0, 64.0, 11.0)
        );
        let hammer = shown_at(State::Move, "hammer_slam", 40);
        assert_eq!(
            slam_centre(&MARGIT, &hammer, 40, &frame),
            DVec3::new(10.0, 64.0, 12.0)
        );
        let cane = shown_at(State::Move, "cane_slam", 29);
        assert_eq!(
            slam_centre(&MARGIT, &cane, 29, &frame),
            DVec3::new(10.0, 64.0, 11.75)
        );
        let mut ambient = AmbientParticles::new(1);
        let mut spell = SpellParticles::new(1);
        let mut random = LegacyRandom::new(1);
        let mut sparks = Sparks {
            ambient: &mut ambient,
            spell: &mut spell,
            random: &mut random,
        };
        sparks.slam(DVec3::new(10.0, 64.0, 12.0));
        let kinds: Vec<(Kind, DVec3)> = ambient.kinds_at().collect();
        assert_eq!(
            kinds
                .iter()
                .filter(|(kind, _)| *kind == Kind::Explosion)
                .count(),
            1
        );
        let poofs: Vec<DVec3> = kinds
            .iter()
            .filter(|(kind, _)| *kind == Kind::Poof)
            .map(|(_, at)| *at)
            .collect();
        assert_eq!(poofs.len(), SLAM_POOFS);
        for at in poofs {
            let flat = DVec3::new(at.x - 10.0, 0.0, at.z - 12.0).length();
            assert!((flat - SLAM_RADIUS).abs() < 1e-9, "a ring: {flat}");
        }
    }
}
