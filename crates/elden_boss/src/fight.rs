use crate::brain::{Act, Brain, HOME, Sense, State};
use crate::data::{Motion, MoveDef, PlayerId};
use crate::hurt::Wounds;
use crate::shape::{self, Frame};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Rules {
    pub(crate) fighters: Vec<PlayerId>,
    pub(crate) shift: bool,
    pub(crate) away: u16,
    pub(crate) stuck: u16,
    pub(crate) stuck_on: Option<PlayerId>,
    pub(crate) pinned: bool,
    pub(crate) homing: bool,
    pub(crate) closest: f64,
    pub(crate) lost: u16,
}

impl Rules {
    pub(crate) fn new() -> Rules {
        Rules {
            fighters: Vec::new(),
            shift: false,
            away: 0,
            stuck: 0,
            stuck_on: None,
            pinned: false,
            homing: false,
            closest: f64::INFINITY,
            lost: 0,
        }
    }
}

impl Brain {
    pub fn awake(&self) -> bool {
        self.state.awake() && !self.rules.homing
    }

    pub fn reset(&mut self, tick: u64) {
        self.enter(State::Return, tick);
        self.phase = 1;
        self.target = None;
        self.fade_to = None;
        self.ready.fill(0);
        self.wounds = Wounds::new(self.def);
        self.wounds.calm = tick;
        self.rules = Rules::new();
    }

    pub(crate) fn open(&mut self, sense: &Sense) {
        let def = self.def;
        let fighters: Vec<PlayerId> = sense
            .players
            .iter()
            .filter(|player| self.in_arena(sense, player))
            .map(|player| player.id)
            .collect();
        let share = if self.wounds.max > 0.0 {
            self.wounds.health / self.wounds.max
        } else {
            1.0
        };
        self.wounds.max = def.health * def.scaling.factor(fighters.len());
        self.wounds.health = self.wounds.max * share;
        self.wounds.stance = def.stance.points;
        self.wounds.calm = sense.tick;
        self.wounds.down = false;
        self.wounds.threat.clear();
        self.wounds.hurt_by.clear();
        self.rules = Rules {
            fighters,
            ..Rules::new()
        };
    }

    pub(crate) fn referee(&mut self, sense: &Sense) {
        let arena = self.def.arena;
        self.refill(sense.tick);
        self.forget(sense.tick);
        if self.state == State::Return {
            self.homeward(sense);
            return;
        }
        if !self.awake() {
            return;
        }
        if self.anyone_within(sense, arena.reset) {
            self.rules.away = 0;
        } else {
            self.rules.away = self.rules.away.saturating_add(1);
            if self.rules.away >= arena.reset_ticks {
                self.reset(sense.tick);
                return;
            }
        }
        if self.rules.shift && self.may_shift() {
            self.change_phase(sense.tick);
            return;
        }
        if self.wounds.stance <= 0.0 && self.breakable() {
            self.kneel(sense.tick);
            return;
        }
        let stuck = self.rules.stuck >= arena.stuck_ticks;
        let room = self.target_seen(sense).and_then(|target| target.room);
        self.rules.pinned = stuck && room.is_none();
        if stuck
            && self.state == State::Idle
            && let Some(room) = room
        {
            self.rules.stuck = 0;
            self.fade_to = Some(room);
            self.enter(State::Fade, sense.tick);
        }
    }

    pub(crate) fn watch(&mut self, sense: &Sense, act: &Act) {
        self.wounds.frame = Frame {
            origin: sense.position + act.motion,
            yaw: act.yaw,
        };
        let target = if self.awake() {
            self.target_seen(sense).copied()
        } else {
            None
        };
        let Some(target) = target else {
            self.rules.stuck = 0;
            self.rules.stuck_on = None;
            return;
        };
        if self.rules.stuck_on != Some(target.id) {
            self.rules.stuck = 0;
            self.rules.stuck_on = Some(target.id);
        }
        let inside = shape::flat_distance(target.feet, sense.anchor) <= self.def.arena.leash;
        let landed = act.hits.iter().any(|hit| hit.player == target.id);
        if inside && !target.reachable && !landed {
            self.rules.stuck = self.rules.stuck.saturating_add(1);
        } else {
            self.rules.stuck = 0;
        }
    }

    pub(crate) fn settle(&mut self, sense: &Sense) -> bool {
        if self.state == State::Fade && self.rules.homing {
            self.rules.homing = false;
            self.enter(State::Asleep, sense.tick);
            return true;
        }
        if self.state == State::Kneel {
            self.wounds.stance = self.def.stance.points;
            self.wounds.calm = sense.tick;
            self.wounds.down = false;
        }
        if self.rules.shift {
            self.change_phase(sense.tick);
            return true;
        }
        if self.wounds.stance <= 0.0 {
            self.kneel(sense.tick);
            return true;
        }
        false
    }

    pub(crate) fn reaches(&self, mv: &MoveDef, distance: f64) -> bool {
        if self.rules.pinned {
            !mv.throws.is_empty()
        } else {
            mv.pick.near <= distance && distance <= mv.pick.far
        }
    }

    pub(crate) fn aloft(&self) -> bool {
        let Some(playing) = self.playing.as_ref() else {
            return false;
        };
        self.def.moves[playing.index].motion.iter().any(|motion| {
            let window = motion.window();
            matches!(motion, Motion::Leap { .. })
                && window.from <= playing.clock
                && playing.clock < window.to
        })
    }

    fn may_shift(&self) -> bool {
        match self.state {
            State::Idle | State::Recover => true,
            State::Move => {
                !self.aloft()
                    && self.playing.as_ref().is_none_or(|playing| {
                        !self.def.moves[playing.index].strikes.iter().any(|strike| {
                            strike.from <= playing.clock && playing.clock <= strike.to
                        })
                    })
            }
            _ => false,
        }
    }

    fn change_phase(&mut self, tick: u64) {
        self.rules.shift = false;
        self.enter(State::Shift, tick);
        self.phase = (self.phase + 1).min(self.def.phases.len() as u8);
    }

    fn homeward(&mut self, sense: &Sense) {
        let distance = shape::flat_distance(sense.position, sense.anchor);
        if distance + HOME <= self.rules.closest {
            self.rules.closest = distance;
            self.rules.lost = 0;
        } else {
            self.rules.lost = self.rules.lost.saturating_add(1);
        }
        if self.rules.lost >= self.def.arena.home_ticks
            && distance > HOME
            && !self.anyone_within(sense, self.def.arena.wake)
        {
            self.rules.homing = true;
            self.rules.lost = 0;
            self.fade_to = Some(sense.anchor);
            self.enter(State::Fade, sense.tick);
        }
    }
}

#[cfg(test)]
mod tests {
    use glam::DVec3;

    use super::*;
    use crate::brain::Seen;
    use crate::brain::tests::{Field, index, player};
    use crate::data::Cue;
    use crate::hurt::{Hurt, Source, Taken};
    use crate::margit::MARGIT;

    fn hurt(field: &Field, source: Source, player: Option<PlayerId>) -> Hurt {
        Hurt {
            source,
            player,
            tick: field.tick,
        }
    }

    fn halve(field: &mut Field) {
        let fire = hurt(field, Source::Environment { damage: 401.0 }, None);
        let taken = field.brain.take(&fire);
        assert!(!taken.died && field.brain.health() <= 400.0);
    }

    fn sounds(act: &Act, event: &str) -> bool {
        act.cues
            .iter()
            .any(|cue| matches!(cue, Cue::Sound(sound) if sound.event == event))
    }

    #[test]
    fn half_health_starts_the_shift_when_the_current_swing_ends() {
        let players = [player(1, 0.0, 2.0)];
        let mut field = Field::new(1);
        field.woken(&players);
        field.brain.begin("cane_slam");
        let slammed = loop {
            let act = field.step(&players);
            if act.clip.clock == 28 && !act.clip.held {
                break act;
            }
        };
        assert_eq!(slammed.hits.len(), 1);
        halve(&mut field);
        let rest: Vec<Act> = (0..2).map(|_| field.step(&players)).collect();
        assert!(rest.iter().all(|act| act.clip.state == State::Move));
        assert_eq!(rest[1].clip.clock, 30);
        let shift: Vec<Act> = (0..30).map(|_| field.step(&players)).collect();
        assert!(
            shift
                .iter()
                .all(|act| act.clip.state == State::Shift && act.clip.phase == 2)
        );
        assert_eq!((shift[0].clip.clock, shift[29].clip.clock), (0, 29));
        assert!(sounds(&shift[20], "entity.elder_guardian.curse"));
        assert!(sounds(&shift[20], "block.beacon.activate"));
        let after = field.step(&players);
        assert_eq!((after.clip.state, after.clip.phase), (State::Idle, 2));
        let mut idle = Field::new(2);
        idle.woken(&players);
        halve(&mut idle);
        assert_eq!(idle.step(&players).clip.state, State::Shift);
        let mut wind_up = Field::new(3);
        wind_up.woken(&players);
        wind_up.brain.begin("cane_slam");
        for _ in 0..5 {
            wind_up.step(&players);
        }
        halve(&mut wind_up);
        let act = wind_up.step(&players);
        assert_eq!((act.clip.state, act.clip.clock), (State::Shift, 0));
        assert!(wind_up.brain.ready[index("cane_slam")] > wind_up.tick);
    }

    #[test]
    fn a_shift_in_a_leap_waits_for_the_landing_strike() {
        let players = [player(1, 0.0, 10.0)];
        let mut field = Field::new(4);
        field.woken(&players);
        field.brain.begin("leap_slam");
        while field.step(&players).clip.clock < 16 {}
        halve(&mut field);
        let mut last = 16;
        loop {
            let act = field.step(&players);
            if act.clip.state == State::Shift {
                break;
            }
            last = act.clip.clock;
        }
        assert_eq!(last, 32);
        assert!(field.position.y.abs() < 1e-9);
    }

    #[test]
    fn stance_does_not_break_during_the_shift() {
        let players = [player(1, 0.0, 2.0)];
        let mut field = Field::new(5);
        field.woken(&players);
        halve(&mut field);
        assert_eq!(field.step(&players).clip.state, State::Shift);
        let from = DVec3::new(0.0, 0.0, 2.0);
        for _ in 0..6 {
            let blade = hurt(&field, Source::Melee { damage: 11.0, from }, Some(1));
            assert!(!field.brain.take(&blade).kneel);
        }
        assert_eq!(field.brain.stance(), 0.0);
        assert!((0..29).all(|_| field.step(&players).clip.state == State::Shift));
        let act = field.step(&players);
        assert_eq!((act.clip.state, act.clip.clock), (State::Kneel, 0));
        assert!((0..79).all(|_| field.step(&players).clip.state == State::Kneel));
        assert_eq!(field.step(&players).clip.state, State::Idle);
        assert_eq!(field.brain.stance(), 80.0);
    }

    #[test]
    fn two_fighters_at_the_wake_give_1280_health_and_keep_the_share() {
        let mut field = Field::new(6);
        field.brain.set_health(400.0);
        let players = [
            player(1, 0.0, 20.0),
            player(2, 30.0, 0.0),
            player(3, 0.0, -45.0),
        ];
        field.step(&players);
        assert_eq!(field.brain.state(), State::Wake);
        assert_eq!(field.brain.max_health(), 1280.0);
        assert!((field.brain.health() - 640.0).abs() < 1e-3);
        assert_eq!(field.brain.fighters(), [1, 2]);
        let more = [
            player(1, 0.0, 20.0),
            player(2, 30.0, 0.0),
            player(3, 0.0, -10.0),
        ];
        for _ in 0..40 {
            field.step(&more);
        }
        assert_eq!(field.brain.max_health(), 1280.0);
        let mut alone = Field::new(7);
        alone.step(&[player(1, 0.0, 20.0)]);
        assert_eq!(alone.brain.max_health(), 800.0);
        assert_eq!(alone.brain.health(), 800.0);
        let mut four = Field::new(8);
        let crowd: Vec<Seen> = (1..=4).map(|id| player(id, id as f64, 10.0)).collect();
        four.step(&crowd);
        assert!((four.brain.max_health() - 2240.0).abs() < 1e-2);
    }

    #[test]
    fn nobody_within_40_blocks_for_5_seconds_resets_the_fight() {
        let near = [player(1, 0.0, 3.0)];
        let gone = [player(1, 0.0, 45.0)];
        let mut field = Field::new(9);
        field.woken(&near);
        halve(&mut field);
        field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 300.0 }, Some(1)));
        while field.brain.state() != State::Idle || field.brain.phase() != 2 {
            field.step(&near);
        }
        field.brain.begin("cane_slam");
        field.play_out(&near);
        assert!(field.brain.ready.iter().any(|&ready| ready > field.tick));
        assert!(field.brain.stance() < 80.0);
        field.position = DVec3::new(10.0, 0.0, 0.0);
        for step in 0..99 {
            assert!(field.step(&gone).clip.awake, "{step}");
        }
        let act = field.step(&gone);
        assert_eq!(act.clip.state, State::Return);
        assert!(!act.clip.awake && act.target.is_none());
        assert_eq!(field.brain.health(), 800.0);
        assert_eq!(field.brain.max_health(), 800.0);
        assert_eq!((field.brain.phase(), field.brain.stance()), (1, 80.0));
        assert!(field.brain.ready.iter().all(|&ready| ready == 0));
        assert!(field.brain.wounds.threat.is_empty());
        assert!(field.brain.fighters().is_empty());
        let back = [player(1, 0.0, 20.0)];
        field.step(&back);
        assert_eq!(field.brain.state(), State::Idle);
        assert_eq!(field.brain.target(), Some(1));
        assert_eq!(field.brain.health(), 800.0);
    }

    #[test]
    fn a_reset_boss_that_gets_no_closer_to_home_for_5_seconds_fades_there() {
        let far = [player(1, 0.0, 60.0)];
        let mut field = Field::new(10);
        field.position = DVec3::new(10.0, 0.0, 0.0);
        field.brain.reset(field.tick);
        assert!((0..100).all(|_| field.step(&far).clip.state == State::Return));
        let mut fade = Vec::new();
        for _ in 0..20 {
            fade.push(field.step(&far));
            let shot = hurt(&field, Source::Bullet { mw2: 30.0 }, Some(1));
            assert_eq!(field.brain.take(&shot), Taken::default());
        }
        assert!(
            fade.iter().all(|act| act.clip.state == State::Fade
                && !act.clip.awake
                && act.target.is_none())
        );
        let swaps: Vec<DVec3> = fade.iter().filter_map(|act| act.fade).collect();
        assert_eq!(swaps, [DVec3::ZERO]);
        assert_eq!(fade[10].fade, Some(DVec3::ZERO));
        assert_eq!(field.step(&far).clip.state, State::Asleep);
        assert_eq!(field.position, DVec3::ZERO);
        assert_eq!(field.brain.health(), 800.0);
    }

    #[test]
    fn an_unreachable_target_for_5_seconds_asks_for_a_fade() {
        let room = DVec3::new(0.0, 0.0, 18.5);
        let stuck = Seen {
            reachable: false,
            room: Some(room),
            ..player(1, 0.0, 20.0)
        };
        let mut field = Field::new(11);
        let mut woke = None;
        let mut faded = None;
        for step in 0..600 {
            let act = field.step(&[stuck]);
            if act.clip.state == State::Wake && woke.is_none() {
                woke = Some(step);
            }
            if act.clip.state == State::Fade && faded.is_none() {
                faded = Some(step);
                assert!(act.clip.awake);
            }
            if let Some(fade) = act.fade {
                assert_eq!(fade, room);
                break;
            }
        }
        let (woke, faded) = (woke.unwrap(), faded.unwrap());
        assert_eq!(woke, 1);
        assert!(faded >= 100, "{faded}");
        assert_eq!(field.position, room);
        let reached = Seen {
            reachable: true,
            ..stuck
        };
        while field.step(&[reached]).clip.state == State::Fade {}
        assert!((0..400).all(|_| field.step(&[reached]).clip.state != State::Fade));
        let mut calm = Field::new(11);
        let free = Seen {
            reachable: true,
            ..stuck
        };
        assert!((0..600).all(|_| calm.step(&[free]).clip.state != State::Fade));
    }

    #[test]
    fn a_target_on_a_pillar_gets_daggers_not_a_fade() {
        let pillar = Seen {
            feet: DVec3::new(0.0, 5.0, 2.0),
            ground: 5.0,
            reachable: false,
            ..player(1, 0.0, 2.0)
        };
        let mut field = Field::new(12);
        let mut before = State::Asleep;
        let mut pinned_moves = Vec::new();
        let mut daggers = 0;
        for _ in 0..900 {
            let pinned = field.brain.rules.pinned;
            let act = field.step(&[pillar]);
            assert_ne!(act.clip.state, State::Fade);
            assert!(act.hits.is_empty());
            if act.clip.state == State::Move && before != State::Move && pinned {
                pinned_moves.push(MARGIT.moves[usize::from(act.clip.index)].key);
            }
            if pinned {
                daggers += act.throws.len();
            }
            before = act.clip.state;
        }
        assert!(pinned_moves.len() >= 4, "{pinned_moves:?}");
        assert!(
            pinned_moves
                .iter()
                .all(|key| ["dagger_throw", "backstep_daggers"].contains(key)),
            "{pinned_moves:?}"
        );
        assert!(daggers >= 6, "{daggers}");
    }

    #[test]
    fn phase_two_idles_6_to_12_ticks_and_cools_moves_by_four_fifths() {
        let players = [player(1, 0.0, 2.0)];
        let mut gaps = Vec::new();
        for seed in 0..60 {
            let mut field = Field::new(seed);
            field.woken(&players);
            halve(&mut field);
            for _ in 0..30 {
                field.step(&players);
            }
            assert_eq!((field.brain.state(), field.brain.phase()), (State::Idle, 2));
            gaps.push(field.brain.gap);
        }
        assert!(gaps.iter().all(|gap| (6..=12).contains(gap)), "{gaps:?}");
        assert!(gaps.iter().min() <= Some(&7) && gaps.iter().max() >= Some(&11));
        for (phase, cooldown) in [(1, 40), (2, 32)] {
            let mut field = Field::new(1);
            field.woken(&players);
            field.brain.phase = phase;
            field.brain.begin("cane_slam");
            field.play_out(&players);
            let end = field.tick - 2;
            assert_eq!(field.brain.ready[index("cane_slam")], end + cooldown);
        }
    }
}
