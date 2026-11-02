use glam::DVec3;

use crate::data::{
    BossDef, BranchTo, ClipDef, Cue, Harm, Motion, MoveDef, PlayerId, Reaction, Shape, Weapon, When,
};
use crate::fight::Rules;
use crate::hurt::Wounds;
use crate::random::SplitMix64;
use crate::shape::{self, Body, Frame};

pub const HOME: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum State {
    Asleep,
    Wake,
    Idle,
    Move,
    Recover,
    Kneel,
    Shift,
    Fade,
    Return,
    Dead,
}

impl State {
    pub const ALL: [State; 10] = [
        State::Asleep,
        State::Wake,
        State::Idle,
        State::Move,
        State::Recover,
        State::Kneel,
        State::Shift,
        State::Fade,
        State::Return,
        State::Dead,
    ];

    pub fn awake(self) -> bool {
        matches!(
            self,
            State::Wake
                | State::Idle
                | State::Move
                | State::Recover
                | State::Kneel
                | State::Shift
                | State::Fade
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seen {
    pub id: PlayerId,
    pub feet: DVec3,
    pub eye: f64,
    pub ground: f64,
    pub reachable: bool,
    pub room: Option<DVec3>,
    pub sight: bool,
}

impl Seen {
    pub fn body(&self) -> Body {
        Body::player(self.feet, self.eye)
    }

    pub fn eyes(&self) -> DVec3 {
        self.feet + DVec3::Y * self.eye
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sense<'a> {
    pub tick: u64,
    pub position: DVec3,
    pub yaw: f64,
    pub on_ground: bool,
    pub anchor: DVec3,
    pub players: &'a [Seen],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Clip {
    pub state: State,
    pub index: u8,
    pub clock: u16,
    pub held: bool,
    pub phase: u8,
    pub awake: bool,
    pub from: u16,
    pub criticals: u8,
}

impl Clip {
    pub fn played(&self, def: &'static BossDef) -> Option<&'static MoveDef> {
        match self.state {
            State::Move | State::Recover => def.moves.get(usize::from(self.index)),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub player: PlayerId,
    pub damage: f32,
    pub harm: Harm,
    pub reaction: Reaction,
    pub shield_off: u16,
    pub weapon: Option<Weapon>,
    pub from: DVec3,
    pub strike: u8,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dagger {
    pub from: DVec3,
    pub velocity: DVec3,
    pub target: Option<PlayerId>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Act {
    pub walk: Option<DVec3>,
    pub yaw: f64,
    pub look: Option<DVec3>,
    pub motion: DVec3,
    pub hits: Vec<Hit>,
    pub throws: Vec<Dagger>,
    pub cues: Vec<Cue>,
    pub fade: Option<DVec3>,
    pub target: Option<PlayerId>,
    pub clip: Clip,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Leap {
    start: DVec3,
    land: DVec3,
}

impl Leap {
    fn at(&self, share: f64, apex: f64) -> DVec3 {
        self.start.lerp(self.land, share) + DVec3::Y * (4.0 * apex * share * (1.0 - share))
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Playing {
    pub(crate) index: usize,
    pub(crate) clock: u16,
    pub(crate) holding: u16,
    frames: Vec<Option<Frame>>,
    struck: Vec<(usize, PlayerId)>,
    leap: Option<Leap>,
}

#[derive(Clone, Debug)]
pub struct Brain {
    pub(crate) def: &'static BossDef,
    pub(crate) random: SplitMix64,
    pub(crate) state: State,
    pub(crate) clock: u16,
    pub(crate) phase: u8,
    pub(crate) target: Option<PlayerId>,
    pub(crate) gap: u16,
    pub(crate) recovery: u16,
    pub(crate) from: u16,
    pub(crate) criticals: u8,
    pub(crate) around: f64,
    pub(crate) ready: Vec<u64>,
    pub(crate) playing: Option<Playing>,
    pub(crate) last: usize,
    pub(crate) fade_to: Option<DVec3>,
    pub(crate) wounds: Wounds,
    pub(crate) rules: Rules,
}

impl Brain {
    pub fn new(def: &'static BossDef, seed: u64) -> Brain {
        Brain {
            def,
            random: SplitMix64::new(seed),
            state: State::Asleep,
            clock: 0,
            phase: 1,
            target: None,
            gap: 0,
            recovery: 0,
            from: 0,
            criticals: 0,
            around: 1.0,
            ready: vec![0; def.moves.len()],
            playing: None,
            last: 0,
            fade_to: None,
            wounds: Wounds::new(def),
            rules: Rules::new(),
        }
    }

    pub fn def(&self) -> &'static BossDef {
        self.def
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn phase(&self) -> u8 {
        self.phase
    }

    pub fn target(&self) -> Option<PlayerId> {
        self.target
    }

    pub fn clip(&self) -> Clip {
        match &self.playing {
            Some(playing) if self.state == State::Move => {
                self.shown(playing.clock, playing.holding > 0)
            }
            _ => self.shown(self.clock, false),
        }
    }

    pub fn gone(&self) -> bool {
        self.state == State::Dead && self.clock >= self.def.clips.death.length
    }

    pub fn begin(&mut self, key: &str) -> bool {
        let Some(index) = self.def.move_index(key) else {
            return false;
        };
        self.playing = None;
        self.start(index);
        true
    }

    pub fn step(&mut self, sense: &Sense) -> Act {
        let mut act = Act {
            walk: None,
            yaw: sense.yaw,
            look: None,
            motion: DVec3::ZERO,
            hits: Vec::new(),
            throws: Vec::new(),
            cues: std::mem::take(&mut self.wounds.cues),
            fade: None,
            target: None,
            clip: self.shown(self.clock, false),
        };
        self.retarget(sense);
        self.referee(sense);
        match self.state {
            State::Asleep => self.asleep(sense, &mut act),
            State::Idle => self.idle(sense, &mut act),
            State::Move => self.play(sense, &mut act),
            State::Recover => self.recover(sense, &mut act),
            State::Return => self.home(sense, &mut act),
            State::Wake | State::Kneel | State::Shift | State::Fade | State::Dead => {
                self.timed(sense, &mut act)
            }
        }
        act.target = self.target;
        if act.clip.awake {
            act.look = self.target_seen(sense).map(Seen::eyes);
        }
        self.watch(sense, &act);
        act
    }

    pub(crate) fn enter(&mut self, state: State, tick: u64) {
        if let Some(playing) = self.playing.take() {
            self.cool(playing.index, tick);
        }
        self.state = state;
        self.clock = 0;
    }

    pub(crate) fn wake(&mut self, sense: &Sense, rise: bool) {
        self.open(sense);
        if rise {
            self.enter(State::Wake, sense.tick);
        } else {
            self.rest(sense);
        }
        self.target = self.chosen(sense);
    }

    pub(crate) fn rest(&mut self, sense: &Sense) {
        if self.settle(sense) {
            return;
        }
        let gap = self.def.phase(self.phase).idle;
        self.state = State::Idle;
        self.clock = 0;
        self.playing = None;
        self.gap = self.random.between(gap.min, gap.max);
        self.around = if self.random.chance(0.5) { 1.0 } else { -1.0 };
        self.target = self.chosen(sense);
    }

    pub(crate) fn nearest(&self, sense: &Sense) -> Option<PlayerId> {
        sense
            .players
            .iter()
            .filter(|player| self.in_arena(sense, player))
            .min_by(|a, b| {
                a.feet
                    .distance_squared(sense.position)
                    .total_cmp(&b.feet.distance_squared(sense.position))
            })
            .map(|player| player.id)
    }

    pub(crate) fn target_seen<'a>(&self, sense: &Sense<'a>) -> Option<&'a Seen> {
        let id = self.target?;
        sense
            .players
            .iter()
            .find(|player| player.id == id && self.in_arena(sense, player))
    }

    pub(crate) fn in_arena(&self, sense: &Sense, player: &Seen) -> bool {
        shape::flat_distance(player.feet, sense.anchor) <= self.def.arena.reset
    }

    pub(crate) fn anyone_within(&self, sense: &Sense, radius: f64) -> bool {
        sense
            .players
            .iter()
            .any(|player| shape::flat_distance(player.feet, sense.anchor) <= radius)
    }

    fn retarget(&mut self, sense: &Sense) {
        if !self.awake() {
            self.target = None;
            return;
        }
        if self.target_seen(sense).is_none() {
            self.target = self.chosen(sense);
        }
    }

    fn shown(&self, clock: u16, held: bool) -> Clip {
        let index = match self.state {
            State::Move | State::Recover => self.last as u8,
            _ => 0,
        };
        Clip {
            state: self.state,
            index,
            clock,
            held,
            phase: self.phase,
            awake: self.awake(),
            from: if self.state == State::Recover {
                self.from
            } else {
                0
            },
            criticals: self.criticals,
        }
    }

    fn asleep(&mut self, sense: &Sense, act: &mut Act) {
        act.clip = self.shown(self.clock, false);
        self.clock = self.clock.wrapping_add(1);
        if self.anyone_within(sense, self.def.arena.wake) {
            self.wake(sense, true);
        }
    }

    fn home(&mut self, sense: &Sense, act: &mut Act) {
        act.clip = self.shown(self.clock, false);
        self.clock = self.clock.wrapping_add(1);
        if self.anyone_within(sense, self.def.arena.wake) {
            self.wake(sense, false);
            return;
        }
        if shape::flat_distance(sense.position, sense.anchor) <= HOME {
            self.enter(State::Asleep, sense.tick);
            return;
        }
        act.walk = Some(sense.anchor);
        act.yaw = self.turn(sense.yaw, sense.position, sense.anchor);
    }

    fn timed(&mut self, sense: &Sense, act: &mut Act) {
        let def = self.def;
        let clip: &ClipDef = match self.state {
            State::Wake => &def.clips.wake,
            State::Kneel => &def.clips.kneel,
            State::Fade => &def.clips.fade,
            State::Dead => &def.clips.death,
            _ => def.shift(self.phase).unwrap_or(&def.clips.idle),
        };
        act.clip = self.shown(self.clock, false);
        if self.state == State::Dead && self.clock >= clip.length {
            return;
        }
        act.cues.extend(
            clip.cues
                .iter()
                .filter(|cue| cue.at == self.clock)
                .map(|cue| cue.cue),
        );
        if self.state == State::Fade && self.clock == def.arena.fade_swap {
            act.fade = self.fade_to.take();
        }
        self.clock += 1;
        if self.clock >= clip.length && self.state != State::Dead {
            self.rest(sense);
        }
    }

    fn idle(&mut self, sense: &Sense, act: &mut Act) {
        let Some(target) = self.target_seen(sense).copied() else {
            act.clip = self.shown(self.clock, false);
            self.clock = self.clock.wrapping_add(1);
            return;
        };
        if self.gap > 0 {
            self.gap -= 1;
        } else if sense.on_ground
            && let Some(index) = self.choose(sense, &target)
        {
            self.start(index);
            self.play(sense, act);
            return;
        }
        act.clip = self.shown(self.clock, false);
        self.clock = self.clock.wrapping_add(1);
        act.yaw = self.turn(sense.yaw, sense.position, target.feet);
        act.walk = Some(self.stalk(sense, &target));
    }

    fn recover(&mut self, sense: &Sense, act: &mut Act) {
        act.clip = self.shown(self.clock, false);
        self.clock += 1;
        if self.clock >= self.recovery {
            self.rest(sense);
        }
    }

    fn choose(&mut self, sense: &Sense, target: &Seen) -> Option<usize> {
        let distance = shape::flat_distance(sense.position, target.feet);
        let angle = shape::yaw_to(sense.position, target.feet)
            .map_or(0.0, |want| shape::wrap(want - sense.yaw).abs());
        let fits: Vec<usize> = self
            .def
            .moves
            .iter()
            .enumerate()
            .filter(|(index, mv)| {
                mv.in_phase(self.phase)
                    && self.ready[*index] <= sense.tick
                    && self.reaches(mv, distance)
                    && angle <= mv.pick.angle
            })
            .map(|(index, _)| index)
            .collect();
        let total: u64 = fits
            .iter()
            .map(|&index| u64::from(self.def.moves[index].pick.weight))
            .sum();
        if total == 0 {
            return None;
        }
        let mut roll = self.random.below(total);
        for index in fits {
            let weight = u64::from(self.def.moves[index].pick.weight);
            if roll < weight {
                return Some(index);
            }
            roll -= weight;
        }
        None
    }

    fn start(&mut self, index: usize) {
        let strikes = self.def.moves[index].strikes.len();
        self.state = State::Move;
        self.clock = 0;
        self.last = index;
        self.playing = Some(Playing {
            index,
            clock: 0,
            holding: 0,
            frames: vec![None; strikes],
            struck: Vec::new(),
            leap: None,
        });
    }

    fn cool(&mut self, index: usize, tick: u64) {
        let cooldown =
            f32::from(self.def.moves[index].pick.cooldown) * self.def.phase(self.phase).cooldown;
        self.ready[index] = tick + cooldown.round().max(0.0) as u64;
    }

    fn play(&mut self, sense: &Sense, act: &mut Act) {
        let Some(mut playing) = self.playing.take() else {
            self.rest(sense);
            act.clip = self.shown(self.clock, false);
            return;
        };
        let def = self.def;
        let mv = &def.moves[playing.index];
        let target = self.target_seen(sense).copied();
        let clock = playing.clock;
        if playing.holding > 0 {
            playing.holding -= 1;
            act.clip = self.shown(clock, true);
            if mv.tracks(clock)
                && let Some(target) = &target
            {
                act.yaw = self.turn(sense.yaw, sense.position, target.feet);
            }
            if playing.holding == 0 {
                playing.clock += 1;
            }
            self.carry(playing, sense);
            return;
        }
        if clock > 0
            && let Some(to) = self.branch(mv, clock, target.as_ref(), sense)
        {
            self.cool(playing.index, sense.tick);
            match to {
                BranchTo::Move(key) => {
                    let index = def.move_index(key).unwrap_or(playing.index);
                    self.start(index);
                    self.play(sense, act);
                }
                BranchTo::End { recovery } => {
                    self.state = State::Recover;
                    self.clock = 0;
                    self.recovery = recovery;
                    self.from = clock;
                    self.recover(sense, act);
                }
            }
            return;
        }
        act.clip = self.shown(clock, false);
        for (index, strike) in mv.strikes.iter().enumerate() {
            if clock < strike.from || clock > strike.to {
                continue;
            }
            let frame = *playing.frames[index].get_or_insert(Frame {
                origin: sense.position,
                yaw: sense.yaw,
            });
            let tick = clock - strike.from;
            let ticks = strike.to - strike.from + 1;
            for player in sense.players {
                if !player.sight || playing.struck.contains(&(index, player.id)) {
                    continue;
                }
                if shape::struck(
                    &strike.shape,
                    &frame,
                    tick,
                    ticks,
                    &player.body(),
                    player.ground,
                ) {
                    playing.struck.push((index, player.id));
                    act.hits.push(Hit {
                        player: player.id,
                        damage: strike.damage,
                        harm: strike.harm,
                        reaction: strike.reaction,
                        shield_off: strike.shield_off,
                        weapon: strike.weapon,
                        from: shape::centre(&strike.shape, &frame),
                        strike: index as u8,
                    });
                }
            }
        }
        if mv.tracks(clock)
            && let Some(target) = &target
        {
            act.yaw = self.turn(sense.yaw, sense.position, target.feet);
        }
        for motion in mv.motion {
            let window = motion.window();
            if window.from <= clock && clock < window.to {
                act.motion += self.motion(
                    mv,
                    motion,
                    clock,
                    &mut playing,
                    sense,
                    target.as_ref(),
                    act.yaw,
                );
            }
        }
        for throw in mv.throws {
            if throw.at == clock
                && throw.phases.contains(&self.phase)
                && self.random.chance(throw.chance)
            {
                act.throws
                    .push(self.dagger(sense, act.yaw, target.as_ref()));
                act.cues.extend(def.cues.dagger_throw.iter().copied());
            }
        }
        act.cues.extend(
            mv.cues
                .iter()
                .filter(|cue| cue.at == clock)
                .map(|cue| cue.cue),
        );
        if let Some(hold) = mv.hold
            && hold.at == clock
        {
            playing.holding = self.random.between(hold.min, hold.max);
        }
        if playing.holding == 0 {
            playing.clock += 1;
        }
        self.carry(playing, sense);
    }

    fn carry(&mut self, playing: Playing, sense: &Sense) {
        if playing.clock >= self.def.moves[playing.index].length {
            self.cool(playing.index, sense.tick);
            self.rest(sense);
        } else {
            self.playing = Some(playing);
        }
    }

    fn branch(
        &mut self,
        mv: &MoveDef,
        clock: u16,
        target: Option<&Seen>,
        sense: &Sense,
    ) -> Option<BranchTo> {
        for branch in mv.branches.iter().filter(|branch| branch.at == clock) {
            let fits = match branch.when {
                When::Always => true,
                When::Phase(phase) => self.phase == phase,
                When::Farther(distance) => target.is_some_and(|target| {
                    shape::flat_distance(sense.position, target.feet) > distance
                }),
            };
            let known = match branch.to {
                BranchTo::Move(key) => self
                    .def
                    .move_index(key)
                    .is_some_and(|index| self.def.moves[index].in_phase(self.phase)),
                BranchTo::End { .. } => true,
            };
            if fits && known && self.random.chance(branch.chance) {
                return Some(branch.to);
            }
        }
        None
    }

    #[allow(clippy::too_many_arguments)]
    fn motion(
        &self,
        mv: &MoveDef,
        motion: &Motion,
        clock: u16,
        playing: &mut Playing,
        sense: &Sense,
        target: Option<&Seen>,
        yaw: f64,
    ) -> DVec3 {
        match *motion {
            Motion::Step { from, to, distance } => self.slide(
                sense,
                shape::forward(yaw) * (distance / f64::from(to - from)),
            ),
            Motion::Hop { from, to, distance } => self.slide(
                sense,
                shape::forward(yaw) * (-distance / f64::from(to - from)),
            ),
            Motion::Leap {
                from,
                to,
                apex,
                near,
                far,
            } => {
                let leap = *playing
                    .leap
                    .get_or_insert_with(|| self.landing(mv, sense, yaw, target, near, far, apex));
                let span = f64::from(to - from);
                let done = f64::from(clock - from);
                leap.at((done + 1.0) / span, apex) - leap.at(done / span, apex)
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn landing(
        &self,
        mv: &MoveDef,
        sense: &Sense,
        yaw: f64,
        target: Option<&Seen>,
        near: f64,
        far: f64,
        apex: f64,
    ) -> Leap {
        let start = sense.position;
        let ahead = mv
            .strikes
            .iter()
            .find_map(|strike| match strike.shape {
                Shape::Circle { ahead, .. } => Some(ahead),
                _ => None,
            })
            .unwrap_or(0.0);
        let wanted = target.map_or(near, |target| {
            shape::flat_distance(start, target.feet) - ahead
        });
        let mut land = self.leash(
            sense.anchor,
            start + shape::forward(yaw) * wanted.clamp(near, far),
        );
        land.y = target.map_or(start.y, |target| {
            target.ground.clamp(start.y - apex, start.y + apex)
        });
        Leap { start, land }
    }

    fn leash(&self, anchor: DVec3, point: DVec3) -> DVec3 {
        let leash = self.def.arena.leash;
        let distance = shape::flat_distance(anchor, point);
        if distance <= leash {
            return point;
        }
        let share = leash / distance;
        DVec3::new(
            anchor.x + (point.x - anchor.x) * share,
            point.y,
            anchor.z + (point.z - anchor.z) * share,
        )
    }

    fn slide(&self, sense: &Sense, delta: DVec3) -> DVec3 {
        let next = shape::flat_distance(sense.anchor, sense.position + delta);
        if next > self.def.arena.leash && next > shape::flat_distance(sense.anchor, sense.position)
        {
            DVec3::ZERO
        } else {
            delta
        }
    }

    fn dagger(&self, sense: &Sense, yaw: f64, target: Option<&Seen>) -> Dagger {
        let daggers = self.def.daggers;
        let from = Frame {
            origin: sense.position,
            yaw,
        }
        .at(daggers.hand);
        let direction = target
            .and_then(|target| (target.body().chest() - from).try_normalize())
            .unwrap_or_else(|| shape::forward(yaw));
        Dagger {
            from,
            velocity: direction * daggers.speed,
            target: target.map(|target| target.id),
        }
    }

    fn stalk(&self, sense: &Sense, target: &Seen) -> DVec3 {
        let stalk = self.def.stalk;
        let away = DVec3::new(
            sense.position.x - target.feet.x,
            0.0,
            sense.position.z - target.feet.z,
        );
        let distance = away.length();
        let point = if distance > stalk.circle {
            target.feet + away * (stalk.stop / distance)
        } else {
            let bearing = away.z.atan2(away.x) + self.around * stalk.step.to_radians();
            let radius = distance.max(stalk.stop);
            target.feet + DVec3::new(bearing.cos(), 0.0, bearing.sin()) * radius
        };
        self.leash(sense.anchor, point)
    }

    fn turn(&self, yaw: f64, from: DVec3, to: DVec3) -> f64 {
        let Some(want) = shape::yaw_to(from, to) else {
            return yaw;
        };
        let delta = shape::wrap(want - yaw).clamp(-self.def.turn, self.def.turn);
        shape::wrap(yaw + delta)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use std::collections::HashSet;
    use std::f64::consts::TAU;

    use super::*;
    use crate::margit::MARGIT;

    pub(crate) const RUN: f64 = 0.265;

    pub(crate) fn player(id: PlayerId, x: f64, z: f64) -> Seen {
        Seen {
            id,
            feet: DVec3::new(x, 0.0, z),
            eye: 1.62,
            ground: 0.0,
            reachable: true,
            room: None,
            sight: true,
        }
    }

    pub(crate) fn index(key: &str) -> usize {
        MARGIT.move_index(key).unwrap()
    }

    pub(crate) struct Field {
        pub(crate) brain: Brain,
        pub(crate) position: DVec3,
        pub(crate) yaw: f64,
        pub(crate) anchor: DVec3,
        pub(crate) tick: u64,
        pub(crate) walks: bool,
    }

    impl Field {
        pub(crate) fn new(seed: u64) -> Field {
            Field {
                brain: Brain::new(&MARGIT, seed),
                position: DVec3::ZERO,
                yaw: 0.0,
                anchor: DVec3::ZERO,
                tick: 1000,
                walks: false,
            }
        }

        pub(crate) fn step(&mut self, players: &[Seen]) -> Act {
            let sense = Sense {
                tick: self.tick,
                position: self.position,
                yaw: self.yaw,
                on_ground: true,
                anchor: self.anchor,
                players,
            };
            let act = self.brain.step(&sense);
            self.position += act.motion;
            if self.walks
                && let Some(walk) = act.walk
            {
                let offset = DVec3::new(walk.x - self.position.x, 0.0, walk.z - self.position.z);
                let length = offset.length();
                if length > 1e-9 {
                    self.position += offset * (length.min(0.15) / length);
                }
            }
            if let Some(fade) = act.fade {
                self.position = fade;
            }
            self.yaw = act.yaw;
            self.tick += 1;
            act
        }

        pub(crate) fn woken(&mut self, players: &[Seen]) {
            for _ in 0..100 {
                if self.step(players).clip.state == State::Idle {
                    return;
                }
            }
            panic!("he never woke")
        }

        pub(crate) fn first_move(&mut self, players: &[Seen]) -> &'static str {
            for _ in 0..400 {
                let act = self.step(players);
                if act.clip.state == State::Move {
                    return MARGIT.moves[usize::from(act.clip.index)].key;
                }
            }
            panic!("no move in 400 ticks")
        }

        pub(crate) fn play_out(&mut self, players: &[Seen]) -> Vec<Act> {
            let mut acts = Vec::new();
            loop {
                let act = self.step(players);
                if act.clip.state != State::Move {
                    return acts;
                }
                acts.push(act);
            }
        }
    }

    #[test]
    fn he_wakes_when_a_player_comes_within_24_blocks_and_rises_for_30_ticks() {
        let mut field = Field::new(1);
        for _ in 0..50 {
            let act = field.step(&[player(1, 0.0, 30.0)]);
            assert_eq!(act.clip.state, State::Asleep);
            assert!(!act.clip.awake && act.target.is_none() && act.look.is_none());
        }
        let near = [player(1, 0.0, 20.0)];
        assert_eq!(field.step(&near).clip.state, State::Asleep);
        let rise: Vec<Act> = (0..30).map(|_| field.step(&near)).collect();
        assert!(
            rise.iter()
                .all(|act| act.clip.state == State::Wake && act.clip.awake)
        );
        assert_eq!(rise[0].clip.clock, 0);
        assert_eq!(rise[29].clip.clock, 29);
        assert!(
            rise[0]
                .cues
                .contains(&Cue::Burst(crate::data::Burst::GoldenMist))
        );
        assert_eq!(rise[0].target, Some(1));
        assert_eq!(field.step(&near).clip.state, State::Idle);
    }

    #[test]
    fn a_player_at_two_blocks_draws_a_close_move_and_one_at_twenty_a_dagger() {
        let close = [player(1, 0.0, 2.0)];
        let mut seen = HashSet::new();
        for seed in 0..200 {
            let key = Field::new(seed).first_move(&close);
            assert!(
                ["cane_combo", "cane_slam", "backstep_daggers", "sword_sweep"].contains(&key),
                "{key}"
            );
            seen.insert(key);
        }
        assert_eq!(seen.len(), 4, "{seen:?}");
        let far = [player(1, 0.0, 20.0)];
        for seed in 0..50 {
            assert_eq!(Field::new(seed).first_move(&far), "dagger_throw");
        }
    }

    #[test]
    fn running_away_at_the_tracking_stop_dodges_the_cane_and_standing_still_does_not() {
        let combo = index("cane_combo");
        let last_track = MARGIT.moves[combo]
            .track
            .iter()
            .flat_map(|window| window.from..=window.to)
            .filter(|&tick| tick < MARGIT.moves[combo].strikes[0].from)
            .max()
            .unwrap();
        assert_eq!(last_track, 4);
        for runs in [false, true] {
            for seed in 0..20 {
                let mut field = Field::new(seed);
                let mut target = player(1, 0.0, 2.5);
                assert!(field.brain.begin("cane_combo"));
                let mut strikes = Vec::new();
                loop {
                    let act = field.step(&[target]);
                    if act.clip.state != State::Move || usize::from(act.clip.index) != combo {
                        break;
                    }
                    strikes.extend(act.hits.iter().map(|hit| hit.strike));
                    if runs && act.clip.clock >= last_track {
                        target.feet.z += RUN;
                    }
                }
                if runs {
                    assert!(strikes.is_empty(), "seed {seed}: {strikes:?}");
                } else {
                    assert_eq!(strikes.first(), Some(&0), "seed {seed}");
                }
            }
        }
    }

    #[test]
    fn a_swing_lands_once_per_player() {
        let players = [player(1, -0.4, 2.0), player(2, 0.4, 2.0)];
        for seed in 0..20 {
            let mut field = Field::new(seed);
            field.brain.begin("cane_slam");
            let acts = field.play_out(&players);
            let window = acts
                .iter()
                .filter(|act| !act.clip.held && (28..=30).contains(&act.clip.clock))
                .count();
            assert_eq!(window, 3);
            let mut hits: Vec<(PlayerId, u8)> = acts
                .iter()
                .flat_map(|act| act.hits.iter().map(|hit| (hit.player, hit.strike)))
                .collect();
            hits.sort();
            assert_eq!(hits, [(1, 0), (2, 0)], "seed {seed}");
            let hit = acts.iter().find_map(|act| act.hits.first()).unwrap();
            assert_eq!((hit.damage, hit.harm), (7.0, Harm::Physical));
            assert_eq!(hit.reaction, Reaction::Stagger);
        }
    }

    #[test]
    fn a_player_he_cannot_see_is_not_struck() {
        let hidden = Seen {
            sight: false,
            ..player(1, -0.4, 2.0)
        };
        let players = [hidden, player(2, 0.4, 2.0)];
        for seed in 0..20 {
            let mut field = Field::new(seed);
            field.brain.begin("cane_slam");
            let hits: Vec<PlayerId> = field
                .play_out(&players)
                .iter()
                .flat_map(|act| act.hits.iter().map(|hit| hit.player))
                .collect();
            assert_eq!(hits, [2], "seed {seed}");
        }
        let mut field = Field::new(3);
        field.brain.begin("cane_slam");
        let mut struck = Vec::new();
        loop {
            let act = field.step(&[Seen {
                sight: field.brain.playing.as_ref().is_some_and(|p| p.clock >= 29),
                ..player(1, 0.0, 2.0)
            }]);
            if act.clip.state != State::Move {
                break;
            }
            struck.extend(act.hits.iter().map(|hit| hit.player));
        }
        assert_eq!(struck, [1], "seen again inside the window");
    }

    #[test]
    fn a_jump_clears_the_hammer_shockwave() {
        for jumping in [false, true] {
            for seed in 0..10 {
                let mut field = Field::new(seed);
                field.brain.phase = 2;
                field.brain.begin("hammer_slam");
                let mut target = player(1, 0.0, 5.0);
                if jumping {
                    target.feet.y = 1.1;
                }
                let hits: Vec<Hit> = field
                    .play_out(&[target])
                    .into_iter()
                    .flat_map(|act| act.hits)
                    .collect();
                if jumping {
                    assert!(hits.is_empty(), "{hits:?}");
                } else {
                    assert_eq!(hits.len(), 1, "{hits:?}");
                    assert_eq!(hits[0].strike, 1);
                    assert_eq!(
                        (hits[0].harm, hits[0].reaction),
                        (Harm::Holy, Reaction::Flinch)
                    );
                    assert_eq!(hits[0].weapon, Some(Weapon::Hammer));
                    assert!((hits[0].from - DVec3::new(0.0, 0.0, 2.0)).length() < 1e-9);
                }
            }
        }
    }

    #[test]
    fn the_dagger_hold_lasts_10_to_30_ticks_and_delays_the_throw() {
        let target = player(1, 0.0, 15.0);
        let mut holds = Vec::new();
        for seed in 0..300 {
            let mut field = Field::new(seed);
            field.brain.begin("dagger_throw");
            let acts = field.play_out(&[target]);
            let held = acts.iter().filter(|act| act.clip.held).count();
            assert!(
                acts.iter()
                    .filter(|act| act.clip.held)
                    .all(|act| act.clip.clock == 10)
            );
            assert!((10..=30).contains(&held), "{held}");
            let thrown: Vec<usize> = acts
                .iter()
                .enumerate()
                .filter(|(_, act)| !act.throws.is_empty())
                .map(|(step, _)| step)
                .collect();
            assert_eq!(thrown, [22 + held]);
            assert_eq!(acts.len(), 36 + held);
            let act = &acts[22 + held];
            let dagger = act.throws[0];
            assert!((dagger.velocity.length() - 1.5).abs() < 1e-9);
            assert!(dagger.from.x > 0.0 && dagger.from.y > 2.0);
            let aim = DVec3::new(0.0, 1.08, 15.0) - dagger.from;
            assert!(dagger.velocity.normalize().dot(aim.normalize()) > 0.999_999);
            assert_eq!(dagger.target, Some(1));
            assert!(act.cues.contains(&MARGIT.cues.dagger_throw[0]));
            holds.push(held);
        }
        assert!(holds.iter().min().unwrap() <= &12, "{holds:?}");
        assert!(holds.iter().max().unwrap() >= &28, "{holds:?}");
    }

    #[test]
    fn the_cane_combo_ends_after_one_two_or_three_swings() {
        let target = player(1, 0.0, 2.5);
        let combo = index("cane_combo");
        let mut counts = [0; 4];
        for seed in 0..400 {
            let mut field = Field::new(seed);
            field.brain.begin("cane_combo");
            let mut swings = 0;
            let mut recovered = false;
            loop {
                let act = field.step(&[target]);
                if act.clip.state == State::Recover {
                    recovered = true;
                }
                if act.clip.state != State::Move || usize::from(act.clip.index) != combo {
                    break;
                }
                swings += act.hits.len();
            }
            assert!((1..=3).contains(&swings), "seed {seed}: {swings}");
            assert_eq!(recovered, swings < 3, "seed {seed}");
            counts[swings] += 1;
        }
        assert!(
            counts[1] > 60 && counts[2] > 50 && counts[3] > 150,
            "{counts:?}"
        );
    }

    #[test]
    fn a_recovery_shows_the_move_clock_it_was_cut_at() {
        let target = player(1, 0.0, 2.5);
        let combo = index("cane_combo");
        let mut cuts = Vec::new();
        for seed in 0..200 {
            let mut field = Field::new(seed);
            assert_eq!(field.brain.clip().state, State::Asleep);
            field.brain.begin("cane_combo");
            let clip = field.brain.clip();
            assert_eq!(
                (clip.state, usize::from(clip.index), clip.clock, clip.held),
                (State::Move, combo, 0, false)
            );
            loop {
                let act = field.step(&[target]);
                match act.clip.state {
                    State::Move => assert_eq!(act.clip.from, 0),
                    State::Recover => cuts.push((act.clip.from, act.clip.clock)),
                    _ => break,
                }
            }
        }
        assert!(cuts.iter().any(|&(from, _)| from == 16), "{cuts:?}");
        assert!(cuts.iter().any(|&(from, _)| from == 26), "{cuts:?}");
        assert!(
            cuts.iter()
                .all(|&(from, clock)| (from == 16 && clock < 14) || (from == 26 && clock < 16)),
            "{cuts:?}"
        );
    }

    #[test]
    fn a_leap_never_lands_outside_the_leash() {
        for (reach, landed) in [(10.0, 9.0), (20.0, 12.0), (3.0, 6.0)] {
            let mut field = Field::new(1);
            let target = player(1, 0.0, reach);
            field.brain.begin("leap_slam");
            let acts = field.play_out(&[target]);
            let mut height = 0.0_f64;
            let mut peak = 0.0_f64;
            for act in &acts {
                height += act.motion.y;
                peak = peak.max(height);
            }
            assert!((peak - 3.0).abs() < 1e-9, "{peak}");
            assert!(
                (field.position.z - landed).abs() < 1e-6,
                "{}",
                field.position
            );
            assert!(field.position.y.abs() < 1e-9);
            let slammed = acts.iter().flat_map(|act| act.hits.iter()).count();
            assert_eq!(slammed, usize::from(reach == 10.0));
        }
        let mut random = SplitMix64::new(99);
        for seed in 0..300 {
            let mut field = Field::new(seed);
            let around = random.unit() * TAU;
            let radius = 24.0 * random.unit().sqrt();
            field.position = DVec3::new(around.cos() * radius, 0.0, around.sin() * radius);
            field.yaw = random.unit() * 360.0 - 180.0;
            let away = random.unit() * TAU;
            let reach = 2.0 + random.unit() * 20.0;
            let target = Seen {
                feet: field.position + DVec3::new(away.cos() * reach, 0.0, away.sin() * reach),
                ..player(1, 0.0, 0.0)
            };
            let mut top = 0.0_f64;
            field.brain.begin("leap_slam");
            loop {
                let act = field.step(&[target]);
                if act.clip.state != State::Move {
                    break;
                }
                top = top.max(field.position.y);
            }
            let out = shape::flat_distance(field.position, field.anchor);
            assert!(out <= MARGIT.arena.leash + 1e-6, "seed {seed}: {out}");
            assert!((top - 3.0).abs() < 1e-9, "seed {seed}: {top}");
        }
    }

    #[test]
    fn a_move_on_cooldown_is_not_picked() {
        let close = [player(1, 0.0, 2.0)];
        let combo = index("cane_combo");
        for seed in 0..100 {
            let mut field = Field::new(seed);
            field.brain.ready[combo] = u64::MAX;
            assert_ne!(field.first_move(&close), "cane_combo");
        }
        for seed in 0..10 {
            let mut field = Field::new(seed);
            field.walks = true;
            let mut last: Vec<Option<u64>> = vec![None; MARGIT.moves.len()];
            let mut before = State::Asleep;
            let mut starts = 0;
            for _ in 0..4000 {
                let tick = field.tick;
                let act = field.step(&close);
                if act.clip.state == State::Move {
                    let played = usize::from(act.clip.index);
                    if before == State::Idle {
                        if let Some(end) = last[played] {
                            let cooldown = u64::from(MARGIT.moves[played].pick.cooldown);
                            assert!(
                                tick - end >= cooldown,
                                "seed {seed}: {} again after {}",
                                MARGIT.moves[played].key,
                                tick - end
                            );
                        }
                        starts += 1;
                    }
                    last[played] = Some(tick);
                }
                before = act.clip.state;
            }
            assert!(starts > 20, "seed {seed}: {starts}");
        }
    }

    #[test]
    fn the_backstep_hops_four_blocks_back_and_throws_two_or_three_daggers() {
        let target = player(1, 0.0, 2.0);
        let mut thirds = [0; 2];
        for phase in [1, 2] {
            for seed in 0..90 {
                let mut field = Field::new(seed);
                field.brain.phase = phase;
                field.brain.begin("backstep_daggers");
                let acts = field.play_out(&[target]);
                assert!((field.position.z + 4.0).abs() < 1e-9, "{}", field.position);
                let daggers: usize = acts.iter().map(|act| act.throws.len()).sum();
                match phase {
                    1 => assert!(daggers == 2 || daggers == 3),
                    _ => assert_eq!(daggers, 3),
                }
                if daggers == 3 {
                    thirds[usize::from(phase - 1)] += 1;
                }
            }
        }
        assert!((15..=45).contains(&thirds[0]), "{thirds:?}");
        assert_eq!(thirds[1], 90);
    }

    #[test]
    fn a_returning_boss_walks_to_his_anchor_and_sleeps_there_or_fights_at_once() {
        let mut field = Field::new(2);
        field.walks = true;
        field.position = DVec3::new(10.0, 0.0, 0.0);
        field.brain.state = State::Return;
        let mut steps = 0;
        while field.brain.state() == State::Return {
            let act = field.step(&[player(1, 0.0, 60.0)]);
            if field.brain.state() == State::Return {
                assert_eq!(act.walk, Some(DVec3::ZERO));
            }
            assert!(!act.clip.awake && act.target.is_none());
            steps += 1;
            assert!(steps < 100);
        }
        assert_eq!(field.brain.state(), State::Asleep);
        assert!(shape::flat_distance(field.position, field.anchor) <= HOME);
        let mut field = Field::new(2);
        field.position = DVec3::new(10.0, 0.0, 0.0);
        field.brain.state = State::Return;
        field.step(&[player(1, 0.0, 20.0)]);
        assert_eq!(field.brain.state(), State::Idle);
        assert_eq!(field.brain.target(), Some(1));
    }

    #[test]
    fn the_same_seed_plays_the_same_moves() {
        fn record(seed: u64) -> Vec<(Clip, usize, usize, DVec3)> {
            let mut field = Field::new(seed);
            field.walks = true;
            (0..4000_u32)
                .map(|step| {
                    let around = f64::from(step) * 0.01;
                    let reach = 3.0 + 12.0 * (f64::from(step) * 0.003).sin().abs();
                    let target = player(1, around.cos() * reach, around.sin() * reach);
                    let act = field.step(&[target]);
                    (act.clip, act.hits.len(), act.throws.len(), field.position)
                })
                .collect()
        }
        let first = record(5);
        assert_eq!(first, record(5));
        assert_ne!(first, record(6));
        let keys: HashSet<u8> = first
            .iter()
            .filter(|entry| entry.0.state == State::Move)
            .map(|entry| entry.0.index)
            .collect();
        assert!(keys.len() >= 4, "{keys:?}");
    }
}
