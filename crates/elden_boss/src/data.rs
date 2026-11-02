use glam::DVec3;

use crate::margit;

pub type PlayerId = u64;

pub const IDS: [&str; 1] = [margit::ID];

static BOSSES: [&BossDef; 1] = [&margit::MARGIT];

pub fn def(id: &str) -> Option<&'static BossDef> {
    BOSSES.iter().copied().find(|def| def.id == id)
}

pub const MW2_BULLET_SCALE: f32 = 0.2;
pub const MW2_BLAST_SCALE: f32 = 0.8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataSource {
    Estimate,
    Baked,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Scaling {
    pub factors: &'static [f32],
    pub extra: f32,
}

impl Scaling {
    pub fn factor(&self, fighters: usize) -> f32 {
        let fighters = fighters.max(1);
        match self.factors.get(fighters - 1) {
            Some(&factor) => factor,
            None => {
                let last = self.factors.last().copied().unwrap_or(1.0);
                last + self.extra * (fighters - self.factors.len()) as f32
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stance {
    pub points: f32,
    pub refill: u16,
    pub kneel: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Factor {
    pub health: f32,
    pub stance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Resist {
    pub bullet: Factor,
    pub blast: Factor,
    pub melee: Factor,
    pub projectile: Factor,
    pub explosion: Factor,
    pub environment: Factor,
    pub kneeling: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Critical {
    pub angle: f64,
    pub reach: f64,
    pub factor: f32,
    pub recovery: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Arena {
    pub wake: f64,
    pub leash: f64,
    pub reset: f64,
    pub bar: f64,
    pub reset_ticks: u16,
    pub stuck_ticks: u16,
    pub home_ticks: u16,
    pub fade_across: f64,
    pub fade_rise: f64,
    pub fade_swap: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stalk {
    pub circle: f64,
    pub stop: f64,
    pub step: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gap {
    pub min: u16,
    pub max: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PhaseDef {
    pub at: f32,
    pub cooldown: f32,
    pub idle: Gap,
    pub shift: Option<ClipDef>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pick {
    pub near: f64,
    pub far: f64,
    pub angle: f64,
    pub weight: u32,
    pub cooldown: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub from: u16,
    pub to: u16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hold {
    pub at: u16,
    pub min: u16,
    pub max: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Motion {
    Step {
        from: u16,
        to: u16,
        distance: f64,
    },
    Hop {
        from: u16,
        to: u16,
        distance: f64,
    },
    Leap {
        from: u16,
        to: u16,
        apex: f64,
        near: f64,
        far: f64,
    },
}

impl Motion {
    pub fn window(&self) -> Window {
        match *self {
            Motion::Step { from, to, .. }
            | Motion::Hop { from, to, .. }
            | Motion::Leap { from, to, .. } => Window { from, to },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Arc {
        reach: f64,
        spread: f64,
        low: f64,
        high: f64,
    },
    Circle {
        ahead: f64,
        radius: f64,
        low: f64,
        high: f64,
    },
    Ring {
        ahead: f64,
        near: f64,
        far: f64,
        low: f64,
        high: f64,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Harm {
    Physical,
    Holy,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Reaction {
    Flinch,
    Stagger,
    Knockdown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Push {
    pub throw: f64,
    pub lift: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reactions {
    pub flinch: Push,
    pub stagger: Push,
    pub knockdown: Push,
}

impl Reactions {
    pub fn push(&self, reaction: Reaction) -> Push {
        match reaction {
            Reaction::Flinch => self.flinch,
            Reaction::Stagger => self.stagger,
            Reaction::Knockdown => self.knockdown,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Weapon {
    Dagger,
    Sword,
    Hammer,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Strike {
    pub from: u16,
    pub to: u16,
    pub shape: Shape,
    pub damage: f32,
    pub harm: Harm,
    pub reaction: Reaction,
    pub shield_off: u16,
    pub weapon: Option<Weapon>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Throw {
    pub at: u16,
    pub phases: &'static [u8],
    pub chance: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Conjure {
    pub weapon: Weapon,
    pub from: u16,
    pub to: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sound {
    pub event: &'static str,
    pub volume: f32,
    pub pitch: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Burst {
    GoldenMist,
    Sparks,
    Slam,
    Crit,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cue {
    Sound(Sound),
    Burst(Burst),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CueKey {
    pub at: u16,
    pub cue: Cue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Pose {
    Ready,
    Breathe,
    CaneBack,
    SwingThrough,
    SwingBack,
    CaneHigh,
    Slam,
    Draw,
    ThrowWindUp,
    Release,
    Hop,
    Crouch,
    Airborne,
    Landing,
    SwordOut,
    SwordBack,
    SweepThrough,
    HammerRaised,
    HammerDown,
    HandRaised,
    Kneel,
    Rise,
    Vanish,
    Collapse,
}

impl Pose {
    pub const ALL: [Pose; 24] = [
        Pose::Ready,
        Pose::Breathe,
        Pose::CaneBack,
        Pose::SwingThrough,
        Pose::SwingBack,
        Pose::CaneHigh,
        Pose::Slam,
        Pose::Draw,
        Pose::ThrowWindUp,
        Pose::Release,
        Pose::Hop,
        Pose::Crouch,
        Pose::Airborne,
        Pose::Landing,
        Pose::SwordOut,
        Pose::SwordBack,
        Pose::SweepThrough,
        Pose::HammerRaised,
        Pose::HammerDown,
        Pose::HandRaised,
        Pose::Kneel,
        Pose::Rise,
        Pose::Vanish,
        Pose::Collapse,
    ];
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PoseKey {
    pub at: u16,
    pub pose: Pose,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum When {
    Always,
    Phase(u8),
    Farther(f64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BranchTo {
    Move(&'static str),
    End { recovery: u16 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Branch {
    pub at: u16,
    pub chance: f32,
    pub when: When,
    pub to: BranchTo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveDef {
    pub key: &'static str,
    pub phases: &'static [u8],
    pub pick: Pick,
    pub length: u16,
    pub track: &'static [Window],
    pub hold: Option<Hold>,
    pub motion: &'static [Motion],
    pub strikes: &'static [Strike],
    pub throws: &'static [Throw],
    pub conjure: &'static [Conjure],
    pub cues: &'static [CueKey],
    pub poses: &'static [PoseKey],
    pub branches: &'static [Branch],
}

impl MoveDef {
    pub fn tracks(&self, clock: u16) -> bool {
        self.track
            .iter()
            .any(|window| window.from <= clock && clock <= window.to)
    }

    pub fn in_phase(&self, phase: u8) -> bool {
        self.phases.contains(&phase)
    }

    pub fn leaps(&self) -> bool {
        self.motion
            .iter()
            .any(|motion| matches!(motion, Motion::Leap { .. }))
    }

    pub fn conjured(&self, clock: u16) -> Option<Weapon> {
        self.conjure
            .iter()
            .find(|conjure| conjure.from <= clock && clock <= conjure.to)
            .map(|conjure| conjure.weapon)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClipDef {
    pub key: &'static str,
    pub length: u16,
    pub poses: &'static [PoseKey],
    pub cues: &'static [CueKey],
    pub conjure: &'static [Conjure],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Clips {
    pub idle: ClipDef,
    pub walk: ClipDef,
    pub wake: ClipDef,
    pub kneel: ClipDef,
    pub fade: ClipDef,
    pub death: ClipDef,
}

impl Clips {
    pub fn all(&self) -> [&ClipDef; 6] {
        [
            &self.idle,
            &self.walk,
            &self.wake,
            &self.kneel,
            &self.fade,
            &self.death,
        ]
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Daggers {
    pub speed: f64,
    pub ticks: u16,
    pub damage: f32,
    pub harm: Harm,
    pub reaction: Reaction,
    pub hand: DVec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Experience {
    pub base: u32,
    pub per_fighter: u32,
}

impl Experience {
    pub fn for_fighters(&self, fighters: usize) -> u32 {
        let more = u32::try_from(fighters.max(1) - 1).unwrap_or(u32::MAX);
        self.base
            .saturating_add(self.per_fighter.saturating_mul(more))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Loot {
    pub item: &'static str,
    pub count: u8,
    pub per_fighter: u8,
}

impl Loot {
    pub fn count_for(&self, fighters: usize) -> u32 {
        let fighters = u32::try_from(fighters.max(1)).unwrap_or(u32::MAX);
        u32::from(self.count).saturating_add(u32::from(self.per_fighter).saturating_mul(fighters))
    }
}

pub const FELLED: &str = "GREAT ENEMY FELLED";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Voices {
    pub ambient: Sound,
    pub hurt: Sound,
    pub death: Sound,
    pub step: Sound,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cues {
    pub critical: &'static [Cue],
    pub dagger_throw: &'static [Cue],
    pub dagger_hit: &'static [Cue],
    pub felled: &'static [Cue],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BossDef {
    pub id: &'static str,
    pub name: &'static str,
    pub source: DataSource,
    pub width: f64,
    pub height: f64,
    pub health: f32,
    pub scaling: Scaling,
    pub stance: Stance,
    pub resist: Resist,
    pub critical: Critical,
    pub speed: f64,
    pub step_height: f64,
    pub turn: f64,
    pub stalk: Stalk,
    pub arena: Arena,
    pub threat: u16,
    pub phases: &'static [PhaseDef],
    pub moves: &'static [MoveDef],
    pub clips: Clips,
    pub daggers: Daggers,
    pub reactions: Reactions,
    pub experience: Experience,
    pub drops: &'static [Loot],
    pub voices: Voices,
    pub cues: Cues,
}

impl BossDef {
    pub fn move_index(&self, key: &str) -> Option<usize> {
        self.moves.iter().position(|mv| mv.key == key)
    }

    pub fn phase(&self, phase: u8) -> &PhaseDef {
        let index = usize::from(phase.max(1)) - 1;
        &self.phases[index.min(self.phases.len() - 1)]
    }

    pub fn shift(&self, phase: u8) -> Option<&ClipDef> {
        self.phases
            .get(usize::from(phase.max(1)) - 1)
            .and_then(|phase| phase.shift.as_ref())
    }
}

pub const TRACKING_STOP: u16 = 8;
pub const WEAPON_LEAD: u16 = 8;
pub const CHECKED_REACH: f64 = 30.0;

pub fn check(def: &BossDef) -> Result<(), String> {
    if def.phases.is_empty() {
        return Err(format!("{}: no phases", def.id));
    }
    if def.moves.is_empty() {
        return Err(format!("{}: no moves", def.id));
    }
    for clip in def.clips.all() {
        check_clip(clip)?;
    }
    if def.clips.kneel.length != def.stance.kneel {
        return Err(format!(
            "{}: the kneel clip lasts {} ticks, the kneel {}",
            def.id, def.clips.kneel.length, def.stance.kneel
        ));
    }
    if def.clips.fade.length <= def.arena.fade_swap {
        return Err(format!(
            "{}: the fade swaps at {}, past its {} ticks",
            def.id, def.arena.fade_swap, def.clips.fade.length
        ));
    }
    for (index, phase) in def.phases.iter().enumerate() {
        if phase.idle.min > phase.idle.max {
            return Err(format!(
                "{}: phase {} idle gap is reversed",
                def.id,
                index + 1
            ));
        }
        match (&phase.shift, index) {
            (Some(_), 0) => {
                return Err(format!("{}: phase 1 has a shift", def.id));
            }
            (None, 1..) => {
                return Err(format!("{}: phase {} has no shift", def.id, index + 1));
            }
            (Some(shift), _) => check_clip(shift)?,
            (None, _) => {}
        }
    }
    for (index, mv) in def.moves.iter().enumerate() {
        if def.moves[..index].iter().any(|other| other.key == mv.key) {
            return Err(format!("{}: two moves are called {}", def.id, mv.key));
        }
        check_move(def, mv)?;
    }
    for phase in 1..=def.phases.len() as u8 {
        let steps = (CHECKED_REACH * 10.0) as u32;
        for step in 0..=steps {
            let distance = f64::from(step) / 10.0;
            let fits = def.moves.iter().any(|mv| {
                mv.in_phase(phase) && mv.pick.near <= distance && distance <= mv.pick.far
            });
            if !fits {
                return Err(format!(
                    "{}: no move of phase {} fits at {} blocks",
                    def.id, phase, distance
                ));
            }
        }
    }
    Ok(())
}

fn check_clip(clip: &ClipDef) -> Result<(), String> {
    if clip.length == 0 {
        return Err(format!("{}: no length", clip.key));
    }
    check_poses(clip.key, clip.length, clip.poses)?;
    for cue in clip.cues {
        if cue.at >= clip.length {
            return Err(format!(
                "{}: a cue at {} is past its {} ticks",
                clip.key, cue.at, clip.length
            ));
        }
    }
    for conjure in clip.conjure {
        if conjure.from > conjure.to || conjure.to >= clip.length {
            return Err(format!(
                "{}: the {:?} shows outside its {} ticks",
                clip.key, conjure.weapon, clip.length
            ));
        }
    }
    Ok(())
}

fn check_poses(key: &str, length: u16, poses: &[PoseKey]) -> Result<(), String> {
    let (Some(first), Some(last)) = (poses.first(), poses.last()) else {
        return Err(format!("{key}: no poses"));
    };
    if first.at != 0 {
        return Err(format!("{key}: the first pose is at {}, not 0", first.at));
    }
    if poses.windows(2).any(|pair| pair[0].at >= pair[1].at) {
        return Err(format!("{key}: poses out of order"));
    }
    if last.at != length {
        return Err(format!(
            "{key}: the last pose is at {}, not at the end {}",
            last.at, length
        ));
    }
    Ok(())
}

fn check_move(def: &BossDef, mv: &MoveDef) -> Result<(), String> {
    let key = mv.key;
    let length = mv.length;
    if length == 0 {
        return Err(format!("{key}: no length"));
    }
    if mv.phases.is_empty()
        || mv
            .phases
            .iter()
            .any(|&phase| phase == 0 || usize::from(phase) > def.phases.len())
    {
        return Err(format!("{key}: unknown phases {:?}", mv.phases));
    }
    if mv.pick.near > mv.pick.far || mv.pick.weight == 0 {
        return Err(format!("{key}: a pick that never fits"));
    }
    for window in mv.track {
        if window.from > window.to || window.to >= length {
            return Err(format!(
                "{key}: tracking {}-{} is outside its {length} ticks",
                window.from, window.to
            ));
        }
    }
    if let Some(hold) = mv.hold {
        if hold.at >= length || hold.min > hold.max {
            return Err(format!(
                "{key}: a hold at {} outside its {length} ticks",
                hold.at
            ));
        }
        if mv
            .strikes
            .iter()
            .any(|strike| strike.from <= hold.at && hold.at <= strike.to)
        {
            return Err(format!("{key}: a hold at {} inside a strike", hold.at));
        }
    }
    for motion in mv.motion {
        let window = motion.window();
        if window.from >= window.to || window.to > length {
            return Err(format!(
                "{key}: motion {}-{} is outside its {length} ticks",
                window.from, window.to
            ));
        }
        if let Motion::Leap { near, far, .. } = *motion
            && near > far
        {
            return Err(format!("{key}: a leap that never lands"));
        }
    }
    for conjure in mv.conjure {
        if conjure.from > conjure.to || conjure.to >= length {
            return Err(format!(
                "{key}: the {:?} shows {}-{}, outside its {length} ticks",
                conjure.weapon, conjure.from, conjure.to
            ));
        }
    }
    let leaps = mv.leaps();
    for (index, strike) in mv.strikes.iter().enumerate() {
        if strike.from > strike.to || strike.to >= length {
            return Err(format!(
                "{key}: strike {index} at {}-{} is past the end of its {length} ticks",
                strike.from, strike.to
            ));
        }
        if !leaps {
            let last = mv
                .track
                .iter()
                .flat_map(|window| window.from..=window.to)
                .filter(|&tick| tick < strike.from)
                .max()
                .map_or(-1, i32::from);
            if i32::from(strike.from) - last < i32::from(TRACKING_STOP) {
                return Err(format!(
                    "{key}: strike {index} at {} is tracked until {last}, less than {TRACKING_STOP} ticks before",
                    strike.from
                ));
            }
        }
        if let Some(weapon) = strike.weapon {
            let shown = mv.conjure.iter().any(|conjure| {
                conjure.weapon == weapon
                    && conjure.from + WEAPON_LEAD <= strike.from
                    && strike.to <= conjure.to
            });
            if !shown {
                return Err(format!(
                    "{key}: strike {index} uses the {weapon:?}, which is not shown from {} ticks before it to its end",
                    WEAPON_LEAD
                ));
            }
        }
    }
    for throw in mv.throws {
        if throw.at >= length {
            return Err(format!(
                "{key}: a throw at {} is past its {length} ticks",
                throw.at
            ));
        }
        if throw.phases.iter().any(|&phase| !mv.in_phase(phase)) {
            return Err(format!("{key}: a throw in a phase the move is not in"));
        }
        if mv.conjured(throw.at) != Some(Weapon::Dagger) {
            return Err(format!(
                "{key}: a throw at {} with no dagger shown",
                throw.at
            ));
        }
    }
    for cue in mv.cues {
        if cue.at >= length {
            return Err(format!(
                "{key}: a cue at {} is past its {length} ticks",
                cue.at
            ));
        }
    }
    check_poses(key, length, mv.poses)?;
    for branch in mv.branches {
        if branch.at == 0 || branch.at >= length {
            return Err(format!(
                "{key}: a branch at {} is at its start or past its {length} ticks",
                branch.at
            ));
        }
        if let BranchTo::Move(to) = branch.to
            && def.move_index(to).is_none()
        {
            return Err(format!("{key}: a branch to the unknown move {to}"));
        }
    }
    Ok(())
}
