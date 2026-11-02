use glam::DVec3;

use crate::brain::{Brain, Sense, State};
use crate::data::{BossDef, Cue, MW2_BLAST_SCALE, MW2_BULLET_SCALE, PlayerId};
use crate::shape::{self, Frame};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Source {
    Bullet { mw2: f32 },
    Blast { mw2: f32 },
    Melee { damage: f32, from: DVec3 },
    Projectile { damage: f32 },
    Explosion { damage: f32 },
    Environment { damage: f32 },
}

impl Source {
    fn dealt(&self, def: &BossDef) -> (f32, f32) {
        let resist = &def.resist;
        let (health, stance, factor) = match *self {
            Source::Bullet { mw2 } => (mw2 * MW2_BULLET_SCALE, mw2, resist.bullet),
            Source::Blast { mw2 } => (mw2 * MW2_BLAST_SCALE, mw2, resist.blast),
            Source::Melee { damage, .. } => (damage, damage, resist.melee),
            Source::Projectile { damage } => (damage, damage, resist.projectile),
            Source::Explosion { damage } => (damage, damage, resist.explosion),
            Source::Environment { damage } => (damage, damage, resist.environment),
        };
        (
            health.max(0.0) * factor.health,
            stance.max(0.0) * factor.stance,
        )
    }
}

pub fn uses_hurt_cooldown(source: &Source) -> bool {
    matches!(source, Source::Environment { .. })
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hurt {
    pub source: Source,
    pub player: Option<PlayerId>,
    pub tick: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Taken {
    pub health: f32,
    pub stance: f32,
    pub critical: bool,
    pub kneel: bool,
    pub died: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Wounds {
    pub(crate) health: f32,
    pub(crate) max: f32,
    pub(crate) stance: f32,
    pub(crate) calm: u64,
    pub(crate) down: bool,
    pub(crate) threat: Vec<(PlayerId, u64, f32)>,
    pub(crate) hurt_by: Vec<PlayerId>,
    pub(crate) frame: Frame,
    pub(crate) cues: Vec<Cue>,
}

impl Wounds {
    pub(crate) fn new(def: &BossDef) -> Wounds {
        Wounds {
            health: def.health,
            max: def.health,
            stance: def.stance.points,
            calm: 0,
            down: false,
            threat: Vec::new(),
            hurt_by: Vec::new(),
            frame: Frame {
                origin: DVec3::ZERO,
                yaw: 0.0,
            },
            cues: Vec::new(),
        }
    }
}

impl Brain {
    pub fn health(&self) -> f32 {
        self.wounds.health
    }

    pub fn max_health(&self) -> f32 {
        self.wounds.max
    }

    pub fn stance(&self) -> f32 {
        self.wounds.stance
    }

    pub fn hurt_by(&self) -> &[PlayerId] {
        &self.wounds.hurt_by
    }

    pub fn fighters(&self) -> Vec<PlayerId> {
        let mut fighters = self.rules.fighters.clone();
        for &player in &self.wounds.hurt_by {
            if !fighters.contains(&player) {
                fighters.push(player);
            }
        }
        fighters
    }

    pub fn set_health(&mut self, health: f32) {
        if health.is_finite() {
            let max = self.wounds.max.max(1.0);
            self.wounds.health = health.clamp(1.0, max);
        }
    }

    pub fn take(&mut self, hurt: &Hurt) -> Taken {
        let mut taken = Taken::default();
        if !self.awake() {
            return taken;
        }
        let def = self.def;
        let (mut health, stance) = hurt.source.dealt(def);
        let kneeling = self.state == State::Kneel;
        if kneeling && self.wounds.down {
            if self.riposte(&hurt.source) {
                health *= def.critical.factor;
                taken.critical = true;
                self.criticals = self.criticals.wrapping_add(1);
                self.wounds.down = false;
                let rise = def.clips.kneel.length.saturating_sub(def.critical.recovery);
                self.clock = self.clock.max(rise);
                self.wounds.cues.extend(def.cues.critical.iter().copied());
            } else {
                health *= def.resist.kneeling;
            }
        }
        taken.health = health.min(self.wounds.health);
        self.wounds.health -= taken.health;
        if let Some(player) = hurt.player
            && taken.health > 0.0
        {
            self.wounds.threat.push((player, hurt.tick, taken.health));
            if !self.wounds.hurt_by.contains(&player) {
                self.wounds.hurt_by.push(player);
            }
        }
        if self.wounds.health <= 0.0 {
            self.wounds.health = 0.0;
            self.rules.shift = false;
            self.enter(State::Dead, hurt.tick);
            taken.died = true;
            return taken;
        }
        if let Some(next) = def.phases.get(usize::from(self.phase))
            && self.wounds.health <= self.wounds.max * next.at
        {
            self.rules.shift = true;
        }
        if !kneeling && stance > 0.0 {
            taken.stance = stance.min(self.wounds.stance);
            self.wounds.stance -= taken.stance;
            self.wounds.calm = hurt.tick;
            if self.wounds.stance <= 0.0 && self.breakable() {
                self.kneel(hurt.tick);
                taken.kneel = true;
            }
        }
        taken
    }

    pub(crate) fn kneel(&mut self, tick: u64) {
        self.enter(State::Kneel, tick);
        self.wounds.stance = 0.0;
        self.wounds.down = true;
    }

    pub(crate) fn breakable(&self) -> bool {
        matches!(self.state, State::Idle | State::Move | State::Recover) && !self.aloft()
    }

    pub(crate) fn refill(&mut self, tick: u64) {
        let stance = self.def.stance;
        if self.state != State::Kneel
            && self.wounds.stance < stance.points
            && tick >= self.wounds.calm + u64::from(stance.refill)
        {
            self.wounds.stance = stance.points;
        }
    }

    pub(crate) fn forget(&mut self, tick: u64) {
        let window = u64::from(self.def.threat);
        self.wounds.threat.retain(|&(_, at, _)| at + window > tick);
    }

    pub(crate) fn chosen(&self, sense: &Sense) -> Option<PlayerId> {
        let window = u64::from(self.def.threat);
        let mut most: Option<(PlayerId, f32)> = None;
        for player in sense
            .players
            .iter()
            .filter(|player| self.in_arena(sense, player))
        {
            let threat: f32 = self
                .wounds
                .threat
                .iter()
                .filter(|&&(id, at, _)| id == player.id && at + window > sense.tick)
                .map(|&(_, _, amount)| amount)
                .sum();
            if threat > 0.0 && most.is_none_or(|(_, top)| threat > top) {
                most = Some((player.id, threat));
            }
        }
        most.map(|(id, _)| id).or_else(|| self.nearest(sense))
    }

    fn riposte(&self, source: &Source) -> bool {
        let Source::Melee { from, .. } = *source else {
            return false;
        };
        let frame = self.wounds.frame;
        let critical = self.def.critical;
        shape::flat_distance(frame.origin, from) <= critical.reach
            && shape::yaw_to(frame.origin, from)
                .is_none_or(|bearing| shape::wrap(bearing - frame.yaw).abs() <= critical.angle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brain::tests::{Field, player};
    use crate::brain::{Act, Seen};
    use crate::data::Burst;

    fn hurt(field: &Field, source: Source, player: PlayerId) -> Hurt {
        Hurt {
            source,
            player: Some(player),
            tick: field.tick,
        }
    }

    fn sword(field: &Field, x: f64, z: f64, player: PlayerId) -> Hurt {
        let from = DVec3::new(x, 0.0, z);
        hurt(field, Source::Melee { damage: 11.0, from }, player)
    }

    fn close(value: f32, wanted: f32) -> bool {
        (value - wanted).abs() < 1e-3
    }

    fn sounds(act: &Act, event: &str) -> bool {
        act.cues
            .iter()
            .any(|cue| matches!(cue, Cue::Sound(sound) if sound.event == event))
    }

    #[test]
    fn an_m4_body_hit_takes_3_health_and_half_a_stance_point() {
        let players = [player(1, 0.0, 10.0)];
        let mut field = Field::new(1);
        field.woken(&players);
        let taken = field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 1));
        assert!(
            close(taken.health, 3.0) && close(taken.stance, 0.5),
            "{taken:?}"
        );
        assert!(!taken.critical && !taken.kneel && !taken.died);
        assert!(close(field.brain.health(), 797.0));
        assert!(close(field.brain.stance(), 79.5));
        let head = field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 42.0 }, 1));
        assert!(close(head.health, 4.2), "{head:?}");
        let barrett = field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 105.0 }, 1));
        assert!(close(barrett.health, 10.5) && close(barrett.stance, 1.75));
    }

    #[test]
    fn a_frag_at_its_centre_takes_30_health_and_10_stance() {
        let players = [player(1, 0.0, 10.0)];
        let mut field = Field::new(1);
        field.woken(&players);
        let frag = field
            .brain
            .take(&hurt(&field, Source::Blast { mw2: 150.0 }, 1));
        assert!(
            close(frag.health, 30.0) && close(frag.stance, 10.0),
            "{frag:?}"
        );
        let tnt = field
            .brain
            .take(&hurt(&field, Source::Explosion { damage: 20.0 }, 1));
        assert!(
            close(tnt.health, 10.0) && close(tnt.stance, 20.0),
            "{tnt:?}"
        );
        let arrow = field
            .brain
            .take(&hurt(&field, Source::Projectile { damage: 9.0 }, 1));
        assert!(
            close(arrow.health, 9.0) && close(arrow.stance, 9.0),
            "{arrow:?}"
        );
        let blade = field.brain.take(&sword(&field, 0.0, 2.0, 1));
        assert!(
            close(blade.health, 11.0) && close(blade.stance, 16.5),
            "{blade:?}"
        );
        assert!(close(field.brain.health(), 800.0 - 60.0));
    }

    #[test]
    fn two_sword_hits_in_one_tick_both_count() {
        let players = [player(1, 0.0, 3.0), player(2, 1.0, 3.0)];
        let mut field = Field::new(2);
        field.woken(&players);
        assert_eq!(field.brain.max_health(), 1280.0);
        let first = field.brain.take(&sword(&field, 0.0, 3.0, 1));
        let second = field.brain.take(&sword(&field, 1.0, 3.0, 2));
        let again = field.brain.take(&sword(&field, 0.0, 3.0, 1));
        assert!(
            [first, second, again]
                .iter()
                .all(|taken| close(taken.health, 11.0))
        );
        assert!(close(field.brain.health(), 1280.0 - 33.0));
        assert_eq!(field.brain.hurt_by(), [1, 2]);
    }

    #[test]
    fn environment_damage_uses_the_hurt_cooldown_and_player_hits_do_not() {
        let from = DVec3::ZERO;
        let made = [
            Source::Bullet { mw2: 30.0 },
            Source::Blast { mw2: 150.0 },
            Source::Melee { damage: 11.0, from },
            Source::Projectile { damage: 9.0 },
            Source::Explosion { damage: 20.0 },
        ];
        assert!(made.iter().all(|source| !uses_hurt_cooldown(source)));
        let fire = Source::Environment { damage: 1.0 };
        assert!(uses_hurt_cooldown(&fire));
        let players = [player(1, 0.0, 10.0)];
        let mut field = Field::new(3);
        field.woken(&players);
        let burnt = field.brain.take(&Hurt {
            source: fire,
            player: None,
            tick: field.tick,
        });
        assert!(close(burnt.health, 1.0) && burnt.stance == 0.0, "{burnt:?}");
        assert_eq!(field.brain.stance(), 80.0);
        assert!(field.brain.hurt_by().is_empty());
        for _ in 0..4 {
            let arrow = field
                .brain
                .take(&hurt(&field, Source::Projectile { damage: 9.0 }, 1));
            assert!(close(arrow.health, 9.0), "{arrow:?}");
            field.step(&players);
        }
        assert!(close(field.brain.health(), 800.0 - 37.0));
    }

    #[test]
    fn a_broken_stance_kneels_80_ticks_and_refills_after_6_quiet_seconds() {
        let players = [player(1, 0.0, 2.0)];
        let mut field = Field::new(4);
        field.woken(&players);
        field.brain.begin("cane_combo");
        for _ in 0..3 {
            field.step(&players);
        }
        let kneels: Vec<bool> = (0..5)
            .map(|_| field.brain.take(&sword(&field, 0.0, 2.0, 1)).kneel)
            .collect();
        assert_eq!(kneels, [false, false, false, false, true]);
        assert_eq!(field.brain.state(), State::Kneel);
        assert_eq!(field.brain.stance(), 0.0);
        let acts: Vec<Act> = (0..80).map(|_| field.step(&players)).collect();
        assert!(
            acts.iter()
                .all(|act| act.clip.state == State::Kneel && act.hits.is_empty())
        );
        assert_eq!((acts[0].clip.clock, acts[79].clip.clock), (0, 79));
        assert!(acts[0].cues.contains(&Cue::Burst(Burst::Crit)));
        assert!(sounds(&acts[0], "entity.ravager.stunned"));
        assert_eq!(field.brain.state(), State::Idle);
        assert_eq!(field.brain.stance(), 80.0);
        let arrow = field
            .brain
            .take(&hurt(&field, Source::Projectile { damage: 9.0 }, 1));
        assert_eq!(arrow.stance, 9.0);
        for _ in 0..120 {
            field.step(&players);
        }
        assert_eq!(field.brain.stance(), 71.0);
        field.step(&players);
        assert_eq!(field.brain.stance(), 80.0);
    }

    #[test]
    fn a_melee_hit_from_the_front_on_a_kneeling_boss_is_a_critical_and_one_from_behind_is_not() {
        let players = [player(1, 0.0, 2.0)];
        let mut field = Field::new(5);
        field.woken(&players);
        let broke = field
            .brain
            .take(&hurt(&field, Source::Projectile { damage: 80.0 }, 1));
        assert!(broke.kneel && !broke.critical, "{broke:?}");
        assert_eq!(field.step(&players).clip.criticals, 0);
        for (x, z) in [(0.0, -2.0), (0.0, 5.0), (3.0, 0.5)] {
            let taken = field.brain.take(&sword(&field, x, z, 1));
            assert!(
                !taken.critical && close(taken.health, 16.5),
                "{x} {z}: {taken:?}"
            );
        }
        let arrow = field
            .brain
            .take(&hurt(&field, Source::Projectile { damage: 10.0 }, 1));
        assert!(!arrow.critical && close(arrow.health, 15.0) && arrow.stance == 0.0);
        let front = field.brain.take(&sword(&field, 1.0, 2.0, 1));
        assert!(
            front.critical && !front.kneel && close(front.health, 33.0),
            "{front:?}"
        );
        let after = field.brain.take(&sword(&field, 1.0, 2.0, 1));
        assert!(!after.critical && close(after.health, 11.0), "{after:?}");
        let act = field.step(&players);
        assert!(sounds(&act, "entity.player.attack.crit"));
        assert!(act.cues.contains(&Cue::Burst(Burst::Crit)));
        assert_eq!((act.clip.state, act.clip.clock), (State::Kneel, 60));
        assert_eq!(act.clip.criticals, 1, "his shown clip counts the critical");
        assert!((0..19).all(|_| field.step(&players).clip.state == State::Kneel));
        assert_eq!(field.step(&players).clip.state, State::Idle);
        assert_eq!(field.brain.stance(), 80.0);
    }

    #[test]
    fn a_stance_broken_in_a_leap_kneels_when_he_lands() {
        let players = [player(1, 0.0, 10.0)];
        let mut field = Field::new(6);
        field.woken(&players);
        field.brain.begin("leap_slam");
        while field.step(&players).clip.clock < 20 {}
        let broke = field
            .brain
            .take(&hurt(&field, Source::Projectile { damage: 80.0 }, 1));
        assert!(!broke.kneel && field.brain.stance() == 0.0, "{broke:?}");
        let mut flight = 0;
        loop {
            let act = field.step(&players);
            if act.clip.state == State::Kneel {
                assert_eq!(act.clip.clock, 0);
                assert!(act.hits.is_empty());
                break;
            }
            assert_eq!(act.clip.state, State::Move);
            flight += 1;
        }
        assert_eq!(flight, 9);
        assert!(field.position.y.abs() < 1e-9 && (field.position.z - 9.0).abs() < 1e-6);
    }

    #[test]
    fn a_killing_blow_kills_him_once_and_he_is_gone_after_20_ticks() {
        let players = [player(1, 0.0, 10.0), player(2, 0.0, 12.0)];
        let mut field = Field::new(7);
        field.woken(&players);
        field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 2));
        let killing = Source::Melee {
            damage: 5000.0,
            from: DVec3::new(0.0, 0.0, 2.0),
        };
        let blow = field.brain.take(&hurt(&field, killing, 1));
        assert!(blow.died && close(blow.health, 1277.0) && blow.stance == 0.0);
        assert_eq!(field.brain.state(), State::Dead);
        assert_eq!(
            field.brain.take(&hurt(&field, killing, 1)),
            Taken::default()
        );
        let acts: Vec<Act> = (0..20).map(|_| field.step(&players)).collect();
        assert!(
            acts.iter().all(|act| act.clip.state == State::Dead
                && !act.clip.awake
                && act.target.is_none())
        );
        assert!(sounds(&acts[0], "entity.evoker.death"));
        assert!(field.brain.gone());
        let mut fighters = field.brain.fighters();
        fighters.sort();
        assert_eq!(fighters, [1, 2]);
    }

    #[test]
    fn an_asleep_or_returning_boss_takes_no_damage() {
        let far = [player(1, 0.0, 30.0)];
        let mut field = Field::new(8);
        field.step(&far);
        assert_eq!(field.brain.state(), State::Asleep);
        let shot = hurt(&field, Source::Bullet { mw2: 30.0 }, 1);
        assert_eq!(field.brain.take(&shot), Taken::default());
        field.brain.reset(field.tick);
        field.position = DVec3::new(10.0, 0.0, 0.0);
        field.step(&far);
        assert_eq!(field.brain.state(), State::Return);
        let blast = hurt(&field, Source::Blast { mw2: 150.0 }, 1);
        assert_eq!(field.brain.take(&blast), Taken::default());
        assert_eq!(field.brain.health(), 800.0);
        assert_eq!(field.brain.stance(), 80.0);
        assert!(field.brain.hurt_by().is_empty());
    }

    #[test]
    fn the_player_who_hurt_him_most_lately_is_his_target() {
        fn after_a_move(field: &mut Field, players: &[Seen]) -> Option<PlayerId> {
            let mut moved = false;
            for _ in 0..400 {
                let state = field.step(players).clip.state;
                if state == State::Move {
                    moved = true;
                } else if moved && state == State::Idle {
                    return field.brain.target();
                }
            }
            panic!("no move ended in 400 ticks")
        }
        let players = [player(1, 0.0, 3.0), player(2, 0.0, 15.0)];
        let mut field = Field::new(9);
        field.woken(&players);
        assert_eq!(field.brain.target(), Some(1));
        for _ in 0..20 {
            field
                .brain
                .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 2));
        }
        assert_eq!(after_a_move(&mut field, &players), Some(2));
        for _ in 0..200 {
            field.step(&players);
        }
        field.brain.take(&sword(&field, 0.0, 3.0, 1));
        assert_eq!(after_a_move(&mut field, &players), Some(1));
        for _ in 0..20 {
            field
                .brain
                .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 2));
        }
        assert_eq!(after_a_move(&mut field, &players), Some(2));
        field.step(&players[..1]);
        assert_eq!(field.brain.target(), Some(1));
        let far = [player(1, 0.0, 3.0), player(2, 0.0, 45.0)];
        for _ in 0..20 {
            field
                .brain
                .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 2));
        }
        assert_eq!(after_a_move(&mut field, &far), Some(1));
    }

    #[test]
    fn the_fighters_are_the_players_at_the_wake_and_everyone_who_hurt_him() {
        let mut field = Field::new(10);
        field.woken(&[player(1, 0.0, 10.0), player(2, 0.0, 20.0)]);
        let later = [
            player(1, 0.0, 10.0),
            player(2, 0.0, 20.0),
            player(3, 0.0, 30.0),
            player(4, 0.0, 35.0),
        ];
        field.step(&later);
        field
            .brain
            .take(&hurt(&field, Source::Bullet { mw2: 30.0 }, 3));
        assert_eq!(field.brain.fighters(), [1, 2, 3]);
        assert_eq!(field.brain.hurt_by(), [3]);
        assert_eq!(field.brain.max_health(), 1280.0);
    }
}
