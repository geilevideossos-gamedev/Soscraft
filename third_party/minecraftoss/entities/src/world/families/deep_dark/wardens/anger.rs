use crate::monster_ai::Target;
use std::cmp::Ordering;

pub const MAX_ANGER: i32 = 150;
pub const CONVERSION_DELAY: i32 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum AngerLevel {
    #[default]
    Calm,
    Agitated,
    Angry,
}

impl AngerLevel {
    pub fn by_anger(anger: i32) -> Self {
        if anger >= Self::Angry.minimum() {
            Self::Angry
        } else if anger >= Self::Agitated.minimum() {
            Self::Agitated
        } else {
            Self::Calm
        }
    }

    pub fn minimum(self) -> i32 {
        match self {
            Self::Calm => 0,
            Self::Agitated => 40,
            Self::Angry => 80,
        }
    }

    pub fn ambient_sound(self) -> &'static str {
        match self {
            Self::Calm => "entity.warden.ambient",
            Self::Agitated => "entity.warden.agitated",
            Self::Angry => "entity.warden.angry",
        }
    }

    pub fn listening_sound(self) -> &'static str {
        match self {
            Self::Calm => "entity.warden.listening",
            Self::Agitated | Self::Angry => "entity.warden.listening_angry",
        }
    }

    pub fn is_angry(self) -> bool {
        self == Self::Angry
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Suspect {
    pub target: Target,
    pub uuid: u128,
    pub anger: i32,
}

impl Suspect {
    fn player(&self) -> bool {
        matches!(self.target, Target::Player(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seen {
    Here { targetable: bool },
    Gone,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Anger {
    pub suspects: Vec<Suspect>,
    pub by_uuid: Vec<(u128, i32)>,
    pub highest: i32,
    pub conversion_delay: i32,
}

fn compare(a: &Suspect, b: &Suspect) -> Ordering {
    let angry = |s: &Suspect| AngerLevel::by_anger(s.anger).is_angry();
    match (angry(a), angry(b)) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }
    match (a.player(), b.player()) {
        (true, false) => return Ordering::Less,
        (false, true) => return Ordering::Greater,
        _ => {}
    }
    b.anger.cmp(&a.anger)
}

impl Anger {
    pub fn loaded(saved: Vec<(u128, i32)>, conversion_delay: i32) -> Self {
        Self {
            by_uuid: saved,
            conversion_delay,
            ..Self::default()
        }
    }

    pub fn anger_of(&self, target: Target) -> i32 {
        self.suspects
            .iter()
            .find(|s| s.target == target)
            .map_or(0, |s| s.anger)
    }

    pub fn active_anger(&self, target: Option<Target>) -> i32 {
        match target {
            Some(target) => self.anger_of(target),
            None => self.highest,
        }
    }

    pub fn top_suspect(&self, targetable: impl Fn(Target) -> bool) -> Option<Target> {
        self.suspects
            .iter()
            .map(|s| s.target)
            .find(|&target| targetable(target))
    }

    pub fn increase(&mut self, target: Target, uuid: u128, amount: i32) -> i32 {
        let anger = match self.suspects.iter_mut().find(|s| s.target == target) {
            Some(suspect) => {
                suspect.anger = MAX_ANGER.min(suspect.anger + amount);
                suspect.anger
            }
            None => {
                let mut anger = MAX_ANGER.min(amount);
                if let Some(index) = self.by_uuid.iter().position(|(id, _)| *id == uuid) {
                    anger += self.by_uuid.remove(index).1;
                }
                self.suspects.push(Suspect {
                    target,
                    uuid,
                    anger,
                });
                anger
            }
        };
        self.sort();
        anger
    }

    pub fn clear(&mut self, target: Target) {
        self.suspects.retain(|s| s.target != target);
        self.sort();
    }

    pub fn tick(&mut self, seen: impl Fn(Target) -> Seen, find: impl Fn(u128) -> Option<Target>) {
        self.conversion_delay -= 1;
        if self.conversion_delay <= 0 {
            self.convert(&find);
            self.conversion_delay = CONVERSION_DELAY;
        }
        self.by_uuid.retain_mut(|(_, anger)| {
            if *anger <= 1 {
                return false;
            }
            *anger -= 1;
            true
        });
        let mut kept = Vec::with_capacity(self.suspects.len());
        for mut suspect in std::mem::take(&mut self.suspects) {
            match seen(suspect.target) {
                Seen::Here { targetable: true } if suspect.anger > 1 => {
                    suspect.anger -= 1;
                    kept.push(suspect);
                }
                Seen::Gone if suspect.anger > 1 => {
                    self.by_uuid.push((suspect.uuid, suspect.anger - 1));
                }
                _ => {}
            }
        }
        self.suspects = kept;
        self.sort();
    }

    fn convert(&mut self, find: &impl Fn(u128) -> Option<Target>) {
        let mut waiting = Vec::with_capacity(self.by_uuid.len());
        for (uuid, anger) in std::mem::take(&mut self.by_uuid) {
            match find(uuid).filter(|&t| !self.suspects.iter().any(|s| s.target == t)) {
                Some(target) => self.suspects.push(Suspect {
                    target,
                    uuid,
                    anger,
                }),
                None => waiting.push((uuid, anger)),
            }
        }
        self.by_uuid = waiting;
    }

    fn sort(&mut self) {
        self.suspects.sort_by(compare);
        self.highest = self.suspects.iter().map(|s| s.anger).max().unwrap_or(0);
    }

    pub fn saved(&self) -> Vec<(u128, i32)> {
        let mut out: Vec<(u128, i32)> = self.suspects.iter().map(|s| (s.uuid, s.anger)).collect();
        out.extend(self.by_uuid.iter().copied());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLAYER: Target = Target::Player(1);
    const COW: Target = Target::Mob(7);
    const ZOMBIE: Target = Target::Mob(8);

    fn here(_: Target) -> Seen {
        Seen::Here { targetable: true }
    }

    fn nobody(_: u128) -> Option<Target> {
        None
    }

    #[test]
    fn anger_levels_match_vanilla() {
        assert_eq!(AngerLevel::by_anger(0), AngerLevel::Calm);
        assert_eq!(AngerLevel::by_anger(39), AngerLevel::Calm);
        assert_eq!(AngerLevel::by_anger(40), AngerLevel::Agitated);
        assert_eq!(AngerLevel::by_anger(79), AngerLevel::Agitated);
        assert_eq!(AngerLevel::by_anger(80), AngerLevel::Angry);
        assert_eq!(AngerLevel::by_anger(150), AngerLevel::Angry);
        assert_eq!(
            AngerLevel::Calm.listening_sound(),
            "entity.warden.listening"
        );
        assert_eq!(
            AngerLevel::Agitated.listening_sound(),
            "entity.warden.listening_angry"
        );
        assert_eq!(AngerLevel::Angry.ambient_sound(), "entity.warden.angry");
    }

    #[test]
    fn anger_rises_by_thirty_five_per_vibration_caps_at_150_and_falls_one_per_second() {
        let mut anger = Anger::default();
        for expected in [35, 70, 105, 140, 150, 150] {
            assert_eq!(anger.increase(PLAYER, 1, 35), expected);
        }
        assert_eq!(anger.highest, 150);
        anger.tick(here, nobody);
        assert_eq!(anger.anger_of(PLAYER), 149);
        for _ in 0..147 {
            anger.tick(here, nobody);
        }
        assert_eq!(anger.anger_of(PLAYER), 2);
        anger.tick(here, nobody);
        assert_eq!(anger.anger_of(PLAYER), 1);
        anger.tick(here, nobody);
        assert!(anger.suspects.is_empty(), "a suspect at one is forgotten");
        assert_eq!(anger.highest, 0);
    }

    #[test]
    fn the_angriest_suspect_is_its_target() {
        let mut anger = Anger::default();
        anger.increase(COW, 7, 70);
        anger.increase(ZOMBIE, 8, 35);
        assert_eq!(anger.top_suspect(|_| true), Some(COW));
        anger.increase(PLAYER, 1, 35);
        assert_eq!(
            anger.top_suspect(|_| true),
            Some(PLAYER),
            "of the calm ones a player comes first"
        );
        anger.increase(COW, 7, 35);
        assert_eq!(
            anger.top_suspect(|_| true),
            Some(COW),
            "an angry one before a calm player"
        );
        assert_eq!(anger.highest, 105);
        assert_eq!(
            anger.top_suspect(|t| t != COW),
            Some(PLAYER),
            "one it cannot target is passed over"
        );
        assert_eq!(anger.active_anger(Some(ZOMBIE)), 35);
        assert_eq!(anger.active_anger(None), 105);
        anger.clear(COW);
        assert_eq!(anger.highest, 35);
    }

    #[test]
    fn a_suspect_that_leaves_is_kept_by_uuid_and_comes_back_angry() {
        let mut anger = Anger::default();
        anger.increase(PLAYER, 11, 100);
        anger.increase(COW, 77, 50);
        let gone = |t: Target| {
            if t == PLAYER {
                Seen::Gone
            } else {
                Seen::Here { targetable: false }
            }
        };
        anger.tick(gone, nobody);
        assert!(anger.suspects.is_empty());
        assert_eq!(
            anger.by_uuid,
            vec![(11, 99)],
            "an untargetable cow is dropped, a player who left is kept"
        );
        assert_eq!(anger.saved(), vec![(11, 99)]);
        anger.tick(here, nobody);
        assert_eq!(anger.by_uuid, vec![(11, 98)]);
        let back = |uuid: u128| (uuid == 11).then_some(PLAYER);
        anger.tick(here, back);
        anger.tick(here, back);
        assert_eq!(anger.anger_of(PLAYER), 96);
        assert!(anger.by_uuid.is_empty());
        let mut loaded = Anger::loaded(vec![(11, 60)], 1);
        assert_eq!(
            loaded.increase(PLAYER, 11, 35),
            95,
            "a new suspect adds its kept anger"
        );
    }
}
