use std::collections::BTreeSet;

use glam::DVec3;

use crate::brain::tests::RUN;
use crate::brain::{Act, Brain, Clip, Seen, Sense, State};
use crate::data::PlayerId;
use crate::hurt::{Hurt, Source};
use crate::margit::MARGIT;
use crate::shape;

const SEED: u64 = 24;
const SEEDS: u64 = 30;
const TICKS: u64 = 6000;
const RIFLEMAN: PlayerId = 1;
const SWORDSMAN: PlayerId = 2;
const EYE: f64 = 1.62;
const WALK: f64 = 0.15;
const FALL: f64 = 0.5;
const BOUND: f64 = 38.0;
const NEAR: f64 = 12.0;
const FAR: f64 = 18.0;
const STRAFE: f64 = 0.15;
const TURN: u64 = 60;
const AIM: u64 = 30;
const BULLET: f32 = 30.0;
const PER_SECOND: u32 = 7;
const MAGAZINE: u32 = 30;
const RELOAD: u64 = 40;
const SWORD: f32 = 11.0;
const CHARGE: u64 = 13;
const REACH: f64 = 3.0;
const CLOSE: f64 = 2.0;
const WIND_UP: f64 = 4.0;
const FRONT: f64 = 1.5;
const SPOT: f64 = 0.5;

#[derive(Debug, Default, PartialEq)]
struct Record {
    moves: BTreeSet<&'static str>,
    phase_two: Option<u64>,
    kneels: u32,
    criticals: u32,
    struck: [u32; 2],
    daggers: [u32; 2],
    targeted: [u32; 2],
    bullets: u32,
    swings: u32,
    farthest: f64,
    returned: bool,
    died: Option<u64>,
    gone: Option<u64>,
}

struct Flying {
    at: DVec3,
    velocity: DVec3,
    age: u16,
}

struct Fight {
    brain: Brain,
    tick: u64,
    position: DVec3,
    yaw: f64,
    rifleman: DVec3,
    swordsman: DVec3,
    strafe: f64,
    cadence: u32,
    shots: u32,
    reloaded: u64,
    swung: u64,
    running: bool,
    shown: Option<Clip>,
    flying: Vec<Flying>,
    record: Record,
}

fn slot(player: PlayerId) -> usize {
    usize::from(player == SWORDSMAN)
}

fn flat(offset: DVec3) -> DVec3 {
    DVec3::new(offset.x, 0.0, offset.z)
}

fn toward(from: DVec3, to: DVec3, most: f64) -> DVec3 {
    let offset = flat(to - from);
    let length = offset.length();
    if length < 1e-9 {
        return from;
    }
    from + offset * (length.min(most) / length)
}

fn bound(point: DVec3) -> DVec3 {
    let out = shape::flat_distance(point, DVec3::ZERO);
    if out <= BOUND {
        return point;
    }
    DVec3::new(point.x * BOUND / out, point.y, point.z * BOUND / out)
}

impl Fight {
    fn new(seed: u64) -> Fight {
        Fight {
            brain: Brain::new(&MARGIT, seed),
            tick: 0,
            position: DVec3::ZERO,
            yaw: 0.0,
            rifleman: DVec3::new(15.0, 0.0, 0.0),
            swordsman: DVec3::new(0.0, 0.0, 6.0),
            strafe: 1.0,
            cadence: 0,
            shots: 0,
            reloaded: 0,
            swung: 0,
            running: false,
            shown: None,
            flying: Vec::new(),
            record: Record::default(),
        }
    }

    fn play(seed: u64) -> Record {
        let mut fight = Fight::new(seed);
        while fight.tick < TICKS && fight.record.gone.is_none() {
            fight.step();
        }
        fight.record
    }

    fn seen(&self) -> [Seen; 2] {
        [(RIFLEMAN, self.rifleman), (SWORDSMAN, self.swordsman)].map(|(id, feet)| Seen {
            id,
            feet,
            eye: EYE,
            ground: 0.0,
            reachable: true,
            room: None,
            sight: true,
        })
    }

    fn step(&mut self) {
        let players = self.seen();
        let sense = Sense {
            tick: self.tick,
            position: self.position,
            yaw: self.yaw,
            on_ground: self.position.y <= 1e-9,
            anchor: DVec3::ZERO,
            players: &players,
        };
        let act = self.brain.step(&sense);
        self.boss(&act);
        self.note(&act);
        self.fly(&players);
        self.flying.extend(act.throws.iter().map(|dagger| Flying {
            at: dagger.from,
            velocity: dagger.velocity,
            age: 0,
        }));
        self.rifle(&act);
        self.sword(&act);
        if self.brain.gone() && self.record.gone.is_none() {
            self.record.gone = Some(self.tick);
        }
        self.shown = Some(act.clip);
        self.tick += 1;
    }

    fn boss(&mut self, act: &Act) {
        self.position += act.motion;
        if act.motion == DVec3::ZERO {
            if let Some(walk) = act.walk {
                self.position = toward(self.position, walk, WALK);
            }
            self.position.y = (self.position.y - FALL).max(0.0);
        }
        if let Some(fade) = act.fade {
            self.position = fade;
        }
        self.yaw = act.yaw;
        let out = shape::flat_distance(self.position, DVec3::ZERO);
        self.record.farthest = self.record.farthest.max(out);
    }

    fn note(&mut self, act: &Act) {
        let clip = act.clip;
        if clip.state == State::Move
            && let Some(mv) = clip.played(&MARGIT)
        {
            self.record.moves.insert(mv.key);
        }
        if clip.phase == 2 && self.record.phase_two.is_none() {
            self.record.phase_two = Some(self.tick);
        }
        if clip.state == State::Kneel && self.shown.is_none_or(|shown| shown.state != State::Kneel)
        {
            self.record.kneels += 1;
        }
        if clip.state == State::Return {
            self.record.returned = true;
        }
        for hit in &act.hits {
            self.record.struck[slot(hit.player)] += 1;
        }
        if let Some(target) = act.target {
            self.record.targeted[slot(target)] += 1;
        }
    }

    fn fly(&mut self, players: &[Seen; 2]) {
        let ticks = MARGIT.daggers.ticks;
        let record = &mut self.record;
        self.flying.retain_mut(|dagger| {
            let to = dagger.at + dagger.velocity;
            let hit = players
                .iter()
                .filter_map(|player| {
                    shape::segment(dagger.at, to, &player.body()).map(|share| (share, player.id))
                })
                .min_by(|a, b| a.0.total_cmp(&b.0));
            if let Some((_, player)) = hit {
                record.daggers[slot(player)] += 1;
                return false;
            }
            dagger.at = to;
            dagger.age += 1;
            dagger.age < ticks && to.y >= 0.0
        });
    }

    fn rifle(&mut self, act: &Act) {
        let away = flat(self.rifleman - self.position);
        let distance = away.length();
        let out = if distance > 1e-9 {
            away / distance
        } else {
            DVec3::X
        };
        if self.tick.is_multiple_of(TURN) {
            self.strafe = -self.strafe;
        }
        let step = if distance < NEAR {
            out * RUN
        } else if distance > FAR {
            -out * RUN
        } else if self.tick % TURN < AIM {
            DVec3::ZERO
        } else {
            DVec3::new(-out.z, 0.0, out.x) * (STRAFE * self.strafe)
        };
        self.rifleman = bound(self.rifleman + step);
        if !act.clip.awake || self.tick < self.reloaded {
            return;
        }
        self.cadence += PER_SECOND;
        if self.cadence < 20 {
            return;
        }
        self.cadence -= 20;
        self.record.bullets += 1;
        self.hurt(Source::Bullet { mw2: BULLET }, RIFLEMAN);
        self.shots += 1;
        if self.shots == MAGAZINE {
            self.shots = 0;
            self.reloaded = self.tick + RELOAD;
        }
    }

    fn sword(&mut self, act: &Act) {
        let clip = act.clip;
        let distance = shape::flat_distance(self.swordsman, self.position);
        match clip.played(&MARGIT).filter(|_| clip.state == State::Move) {
            Some(mv) => {
                let fresh = self
                    .shown
                    .is_none_or(|shown| shown.state != State::Move || shown.index != clip.index);
                if fresh && !mv.strikes.is_empty() && distance <= WIND_UP {
                    self.running = true;
                }
                if !clip.held && mv.strikes.iter().any(|strike| clip.clock == strike.to + 1) {
                    self.running = false;
                }
            }
            None => self.running = false,
        }
        self.swordsman = bound(if self.running {
            let away = flat(self.swordsman - self.position);
            let length = away.length().max(1e-9);
            self.swordsman + away * (RUN / length)
        } else if clip.state == State::Kneel {
            let front = self.position + shape::forward(self.yaw) * FRONT;
            toward(self.swordsman, front, RUN)
        } else {
            let gap = (distance - CLOSE).max(0.0);
            toward(self.swordsman, self.position, gap.min(RUN))
        });
        let distance = shape::flat_distance(self.swordsman, self.position);
        let front = self.position + shape::forward(self.yaw) * FRONT;
        let kneeling = clip.state == State::Kneel;
        if self.running
            || !clip.awake
            || self.tick < self.swung
            || distance > REACH
            || (kneeling && shape::flat_distance(self.swordsman, front) > SPOT)
        {
            return;
        }
        self.swung = self.tick + CHARGE;
        self.record.swings += 1;
        let from = self.swordsman;
        self.hurt(
            Source::Melee {
                damage: SWORD,
                from,
            },
            SWORDSMAN,
        );
    }

    fn hurt(&mut self, source: Source, player: PlayerId) {
        let taken = self.brain.take(&Hurt {
            source,
            player: Some(player),
            tick: self.tick,
        });
        if taken.critical {
            self.record.criticals += 1;
        }
        if taken.died {
            self.record.died = Some(self.tick);
        }
    }
}

#[test]
fn a_whole_margit_fight_shows_every_move_both_phases_a_kneel_and_ends() {
    let record = Fight::play(SEED);
    let every: BTreeSet<&str> = MARGIT.moves.iter().map(|mv| mv.key).collect();
    assert_eq!(record.moves, every, "{record:?}");
    assert!(record.phase_two.is_some(), "{record:?}");
    assert!(record.kneels >= 1 && record.criticals >= 1, "{record:?}");
    for player in 0..2 {
        assert!(
            record.struck[player] + record.daggers[player] >= 1,
            "{record:?}"
        );
    }
    let died = record.died.expect("he never died");
    assert!(died < TICKS, "{record:?}");
    assert_eq!(
        record.gone,
        Some(died + u64::from(MARGIT.clips.death.length))
    );
    assert!(!record.returned, "{record:?}");
    assert!(record.farthest <= MARGIT.arena.leash + 1e-6, "{record:?}");
    assert_eq!(Fight::play(SEED), record);
    assert_ne!(Fight::play(SEED + 1).died, record.died);
}

#[test]
fn every_seed_of_the_whole_fight_reaches_phase_two_kneels_and_dies_inside_his_arena() {
    for seed in 0..SEEDS {
        let record = Fight::play(seed);
        assert!(record.phase_two.is_some(), "seed {seed}: {record:?}");
        assert!(
            record.kneels >= 1 && record.criticals >= 1,
            "seed {seed}: {record:?}"
        );
        assert!(
            record.targeted.iter().all(|&ticks| ticks > 0),
            "seed {seed}: {record:?}"
        );
        assert!(
            record.died.is_some_and(|died| died < TICKS),
            "seed {seed}: {record:?}"
        );
        assert!(!record.returned, "seed {seed}: {record:?}");
        assert!(
            record.farthest <= MARGIT.arena.leash + 1e-6,
            "seed {seed}: {record:?}"
        );
    }
}
