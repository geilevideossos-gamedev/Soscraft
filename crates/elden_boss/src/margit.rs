use glam::DVec3;

use crate::data::{
    Arena, BossDef, Branch, BranchTo, Burst, ClipDef, Clips, Conjure, Critical, Cue, CueKey, Cues,
    Daggers, DataSource, Experience, Factor, Gap, Harm, Hold, Loot, Motion, MoveDef, PhaseDef,
    Pick, Pose, PoseKey, Reaction, Resist, Scaling, Shape, Sound, Stalk, Stance, Strike, Throw,
    Voices, Weapon, When, Window,
};

pub const ID: &str = "iw4l:margit";

const fn sound(event: &'static str, volume: f32, pitch: f32) -> Cue {
    Cue::Sound(Sound {
        event,
        volume,
        pitch,
    })
}

const fn cue(at: u16, cue: Cue) -> CueKey {
    CueKey { at, cue }
}

const fn burst(at: u16, burst: Burst) -> CueKey {
    CueKey {
        at,
        cue: Cue::Burst(burst),
    }
}

const fn pose(at: u16, pose: Pose) -> PoseKey {
    PoseKey { at, pose }
}

const fn window(from: u16, to: u16) -> Window {
    Window { from, to }
}

const SWING: Cue = sound("entity.player.attack.sweep", 1.0, 0.6);
const CHIME: Cue = sound("block.amethyst_block.chime", 1.0, 1.6);
const BEACON: Cue = sound("block.beacon.activate", 0.6, 1.6);
const ROAR: Cue = sound("entity.ravager.roar", 1.0, 0.8);
const SMASH: Cue = sound("item.mace.smash_ground", 1.0, 0.8);
const SMASH_HEAVY: Cue = sound("item.mace.smash_ground_heavy", 1.0, 0.8);
const BOOM: Cue = sound("entity.generic.explode", 0.3, 0.6);
const MIST: Cue = sound("entity.illusioner.mirror_move", 1.0, 0.8);
const STUNNED: Cue = sound("entity.ravager.stunned", 1.0, 1.0);
const CURSE: Cue = sound("entity.elder_guardian.curse", 1.0, 1.0);
const DEATH: Cue = sound("entity.evoker.death", 1.0, 0.6);

const CANE: Shape = Shape::Arc {
    reach: 3.5,
    spread: 100.0,
    low: -1.0,
    high: 2.5,
};

const SHIELD_OFF: u16 = 100;

const MOVE: MoveDef = MoveDef {
    key: "",
    phases: &[1, 2],
    pick: Pick {
        near: 0.0,
        far: 4.0,
        angle: 60.0,
        weight: 1,
        cooldown: 0,
    },
    length: 1,
    track: &[],
    hold: None,
    motion: &[],
    strikes: &[],
    throws: &[],
    conjure: &[],
    cues: &[],
    poses: &[],
    branches: &[],
};

static MOVES: [MoveDef; 8] = [
    MoveDef {
        key: "cane_combo",
        pick: Pick {
            near: 0.0,
            far: 4.0,
            angle: 60.0,
            weight: 30,
            cooldown: 20,
        },
        length: 70,
        track: &[window(0, 4), window(12, 14), window(26, 36)],
        hold: Some(Hold {
            at: 34,
            min: 0,
            max: 10,
        }),
        strikes: &[
            Strike {
                from: 12,
                to: 14,
                shape: CANE,
                damage: 4.0,
                harm: Harm::Physical,
                reaction: Reaction::Flinch,
                shield_off: 0,
                weapon: None,
            },
            Strike {
                from: 22,
                to: 24,
                shape: CANE,
                damage: 4.0,
                harm: Harm::Physical,
                reaction: Reaction::Flinch,
                shield_off: 0,
                weapon: None,
            },
            Strike {
                from: 44,
                to: 46,
                shape: Shape::Arc {
                    reach: 3.5,
                    spread: 120.0,
                    low: -1.0,
                    high: 2.5,
                },
                damage: 6.0,
                harm: Harm::Physical,
                reaction: Reaction::Stagger,
                shield_off: 0,
                weapon: None,
            },
        ],
        cues: &[cue(11, SWING), cue(21, SWING), cue(43, SWING)],
        poses: &[
            pose(0, Pose::Ready),
            pose(8, Pose::CaneBack),
            pose(13, Pose::SwingThrough),
            pose(23, Pose::SwingBack),
            pose(32, Pose::CaneHigh),
            pose(34, Pose::CaneHigh),
            pose(45, Pose::Slam),
            pose(58, Pose::Slam),
            pose(70, Pose::Ready),
        ],
        branches: &[
            Branch {
                at: 16,
                chance: 0.25,
                when: When::Always,
                to: BranchTo::End { recovery: 14 },
            },
            Branch {
                at: 26,
                chance: 0.3,
                when: When::Always,
                to: BranchTo::End { recovery: 16 },
            },
            Branch {
                at: 26,
                chance: 0.4,
                when: When::Phase(2),
                to: BranchTo::Move("hammer_slam"),
            },
            Branch {
                at: 52,
                chance: 0.3,
                when: When::Always,
                to: BranchTo::Move("sword_sweep"),
            },
        ],
        ..MOVE
    },
    MoveDef {
        key: "cane_slam",
        pick: Pick {
            near: 0.0,
            far: 4.0,
            angle: 45.0,
            weight: 20,
            cooldown: 40,
        },
        length: 54,
        track: &[window(0, 20)],
        hold: Some(Hold {
            at: 20,
            min: 0,
            max: 12,
        }),
        strikes: &[Strike {
            from: 28,
            to: 30,
            shape: Shape::Arc {
                reach: 3.5,
                spread: 60.0,
                low: -1.0,
                high: 2.5,
            },
            damage: 7.0,
            harm: Harm::Physical,
            reaction: Reaction::Stagger,
            shield_off: 0,
            weapon: None,
        }],
        cues: &[
            cue(27, SWING),
            cue(29, SMASH),
            cue(29, BOOM),
            burst(29, Burst::Slam),
        ],
        poses: &[
            pose(0, Pose::Ready),
            pose(12, Pose::CaneHigh),
            pose(20, Pose::CaneHigh),
            pose(29, Pose::Slam),
            pose(42, Pose::Slam),
            pose(54, Pose::Ready),
        ],
        ..MOVE
    },
    MoveDef {
        key: "dagger_throw",
        pick: Pick {
            near: 5.0,
            far: 30.0,
            angle: 90.0,
            weight: 25,
            cooldown: 60,
        },
        length: 36,
        track: &[window(0, 18)],
        hold: Some(Hold {
            at: 10,
            min: 10,
            max: 30,
        }),
        throws: &[Throw {
            at: 22,
            phases: &[1, 2],
            chance: 1.0,
        }],
        conjure: &[Conjure {
            weapon: Weapon::Dagger,
            from: 2,
            to: 22,
        }],
        cues: &[cue(2, CHIME), cue(2, BEACON), burst(2, Burst::Sparks)],
        poses: &[
            pose(0, Pose::Ready),
            pose(4, Pose::Draw),
            pose(10, Pose::ThrowWindUp),
            pose(22, Pose::Release),
            pose(28, Pose::Release),
            pose(36, Pose::Ready),
        ],
        ..MOVE
    },
    MoveDef {
        key: "backstep_daggers",
        pick: Pick {
            near: 0.0,
            far: 5.0,
            angle: 90.0,
            weight: 15,
            cooldown: 100,
        },
        length: 60,
        track: &[window(12, 26)],
        motion: &[Motion::Hop {
            from: 2,
            to: 12,
            distance: 4.0,
        }],
        throws: &[
            Throw {
                at: 22,
                phases: &[1, 2],
                chance: 1.0,
            },
            Throw {
                at: 30,
                phases: &[1, 2],
                chance: 1.0,
            },
            Throw {
                at: 38,
                phases: &[1],
                chance: 1.0 / 3.0,
            },
            Throw {
                at: 38,
                phases: &[2],
                chance: 1.0,
            },
        ],
        conjure: &[Conjure {
            weapon: Weapon::Dagger,
            from: 14,
            to: 38,
        }],
        cues: &[cue(14, CHIME), cue(14, BEACON), burst(14, Burst::Sparks)],
        poses: &[
            pose(0, Pose::Ready),
            pose(2, Pose::Crouch),
            pose(6, Pose::Hop),
            pose(12, Pose::Landing),
            pose(18, Pose::ThrowWindUp),
            pose(22, Pose::Release),
            pose(26, Pose::ThrowWindUp),
            pose(30, Pose::Release),
            pose(34, Pose::ThrowWindUp),
            pose(38, Pose::Release),
            pose(48, Pose::Release),
            pose(60, Pose::Ready),
        ],
        ..MOVE
    },
    MoveDef {
        key: "leap_slam",
        pick: Pick {
            near: 8.0,
            far: 16.0,
            angle: 30.0,
            weight: 30,
            cooldown: 120,
        },
        length: 60,
        track: &[window(0, 14)],
        motion: &[Motion::Leap {
            from: 14,
            to: 30,
            apex: 3.0,
            near: 6.0,
            far: 12.0,
        }],
        strikes: &[Strike {
            from: 30,
            to: 32,
            shape: Shape::Circle {
                ahead: 1.0,
                radius: 1.5,
                low: -1.0,
                high: 2.5,
            },
            damage: 8.0,
            harm: Harm::Physical,
            reaction: Reaction::Knockdown,
            shield_off: 0,
            weapon: None,
        }],
        cues: &[
            cue(4, ROAR),
            cue(30, SMASH),
            cue(30, BOOM),
            burst(30, Burst::Slam),
        ],
        poses: &[
            pose(0, Pose::Ready),
            pose(8, Pose::Crouch),
            pose(14, Pose::Crouch),
            pose(20, Pose::Airborne),
            pose(30, Pose::Landing),
            pose(44, Pose::Landing),
            pose(60, Pose::Ready),
        ],
        branches: &[Branch {
            at: 40,
            chance: 0.3,
            when: When::Phase(2),
            to: BranchTo::Move("sword_sweep"),
        }],
        ..MOVE
    },
    MoveDef {
        key: "sword_sweep",
        pick: Pick {
            near: 0.0,
            far: 5.0,
            angle: 60.0,
            weight: 15,
            cooldown: 40,
        },
        length: 48,
        track: &[window(0, 14)],
        strikes: &[Strike {
            from: 22,
            to: 25,
            shape: Shape::Arc {
                reach: 4.5,
                spread: 180.0,
                low: -1.0,
                high: 2.0,
            },
            damage: 7.0,
            harm: Harm::Holy,
            reaction: Reaction::Stagger,
            shield_off: 0,
            weapon: Some(Weapon::Sword),
        }],
        conjure: &[Conjure {
            weapon: Weapon::Sword,
            from: 0,
            to: 30,
        }],
        cues: &[
            cue(0, CHIME),
            cue(0, BEACON),
            burst(0, Burst::Sparks),
            cue(21, SWING),
        ],
        poses: &[
            pose(0, Pose::Ready),
            pose(8, Pose::SwordOut),
            pose(16, Pose::SwordBack),
            pose(22, Pose::SwordBack),
            pose(25, Pose::SweepThrough),
            pose(36, Pose::SweepThrough),
            pose(48, Pose::Ready),
        ],
        branches: &[Branch {
            at: 30,
            chance: 1.0,
            when: When::Farther(6.0),
            to: BranchTo::Move("dagger_throw"),
        }],
        ..MOVE
    },
    MoveDef {
        key: "hammer_slam",
        phases: &[2],
        pick: Pick {
            near: 0.0,
            far: 4.0,
            angle: 45.0,
            weight: 25,
            cooldown: 80,
        },
        length: 76,
        track: &[window(0, 32)],
        hold: Some(Hold {
            at: 30,
            min: 0,
            max: 8,
        }),
        strikes: &[
            Strike {
                from: 40,
                to: 42,
                shape: Shape::Arc {
                    reach: 3.5,
                    spread: 70.0,
                    low: -1.0,
                    high: 2.5,
                },
                damage: 11.0,
                harm: Harm::Physical,
                reaction: Reaction::Knockdown,
                shield_off: SHIELD_OFF,
                weapon: Some(Weapon::Hammer),
            },
            Strike {
                from: 40,
                to: 46,
                shape: Shape::Ring {
                    ahead: 2.0,
                    near: 0.5,
                    far: 3.5,
                    low: -1.0,
                    high: 1.0,
                },
                damage: 4.0,
                harm: Harm::Holy,
                reaction: Reaction::Flinch,
                shield_off: 0,
                weapon: Some(Weapon::Hammer),
            },
        ],
        conjure: &[Conjure {
            weapon: Weapon::Hammer,
            from: 0,
            to: 58,
        }],
        cues: &[
            cue(0, CHIME),
            cue(0, BEACON),
            burst(0, Burst::Sparks),
            cue(40, SMASH_HEAVY),
            cue(40, BOOM),
            burst(40, Burst::Slam),
        ],
        poses: &[
            pose(0, Pose::Ready),
            pose(10, Pose::HammerRaised),
            pose(30, Pose::HammerRaised),
            pose(41, Pose::HammerDown),
            pose(58, Pose::HammerDown),
            pose(76, Pose::Ready),
        ],
        ..MOVE
    },
    MoveDef {
        key: "hammer_leap",
        phases: &[2],
        pick: Pick {
            near: 8.0,
            far: 16.0,
            angle: 30.0,
            weight: 20,
            cooldown: 160,
        },
        length: 70,
        track: &[window(0, 16)],
        motion: &[Motion::Leap {
            from: 16,
            to: 34,
            apex: 4.0,
            near: 6.0,
            far: 12.0,
        }],
        strikes: &[
            Strike {
                from: 34,
                to: 36,
                shape: Shape::Circle {
                    ahead: 1.5,
                    radius: 1.5,
                    low: -1.0,
                    high: 2.5,
                },
                damage: 12.0,
                harm: Harm::Physical,
                reaction: Reaction::Knockdown,
                shield_off: SHIELD_OFF,
                weapon: Some(Weapon::Hammer),
            },
            Strike {
                from: 34,
                to: 40,
                shape: Shape::Ring {
                    ahead: 1.5,
                    near: 0.5,
                    far: 4.0,
                    low: -1.0,
                    high: 1.0,
                },
                damage: 5.0,
                harm: Harm::Holy,
                reaction: Reaction::Flinch,
                shield_off: 0,
                weapon: Some(Weapon::Hammer),
            },
        ],
        conjure: &[Conjure {
            weapon: Weapon::Hammer,
            from: 0,
            to: 52,
        }],
        cues: &[
            cue(0, CHIME),
            cue(0, BEACON),
            burst(0, Burst::Sparks),
            cue(6, ROAR),
            cue(34, SMASH_HEAVY),
            cue(34, BOOM),
            burst(34, Burst::Slam),
        ],
        poses: &[
            pose(0, Pose::Ready),
            pose(8, Pose::HammerRaised),
            pose(16, Pose::Crouch),
            pose(22, Pose::Airborne),
            pose(34, Pose::HammerDown),
            pose(52, Pose::HammerDown),
            pose(70, Pose::Ready),
        ],
        ..MOVE
    },
];

static PHASES: [PhaseDef; 2] = [
    PhaseDef {
        at: 1.0,
        cooldown: 1.0,
        idle: Gap { min: 10, max: 20 },
        shift: None,
    },
    PhaseDef {
        at: 0.5,
        cooldown: 0.8,
        idle: Gap { min: 6, max: 12 },
        shift: Some(ClipDef {
            key: "shift",
            length: 30,
            poses: &[
                pose(0, Pose::Ready),
                pose(10, Pose::HandRaised),
                pose(20, Pose::HandRaised),
                pose(30, Pose::Ready),
            ],
            cues: &[
                cue(8, CHIME),
                burst(8, Burst::Sparks),
                cue(20, CURSE),
                cue(20, BEACON),
                burst(20, Burst::GoldenMist),
            ],
            conjure: &[Conjure {
                weapon: Weapon::Hammer,
                from: 8,
                to: 29,
            }],
        }),
    },
];

pub static MARGIT: BossDef = BossDef {
    id: ID,
    name: "Margit, the Fell Omen",
    source: DataSource::Estimate,
    width: 1.4,
    height: 3.0,
    health: 800.0,
    scaling: Scaling {
        factors: &[1.0, 1.6, 2.2, 2.8],
        extra: 0.6,
    },
    stance: Stance {
        points: 80.0,
        refill: 120,
        kneel: 80,
    },
    resist: Resist {
        bullet: Factor {
            health: 0.5,
            stance: 1.0 / 60.0,
        },
        blast: Factor {
            health: 0.25,
            stance: 1.0 / 15.0,
        },
        melee: Factor {
            health: 1.0,
            stance: 1.5,
        },
        projectile: Factor {
            health: 1.0,
            stance: 1.0,
        },
        explosion: Factor {
            health: 0.5,
            stance: 1.0,
        },
        environment: Factor {
            health: 1.0,
            stance: 0.0,
        },
        kneeling: 1.5,
    },
    critical: Critical {
        angle: 60.0,
        reach: 4.0,
        factor: 3.0,
        recovery: 20,
    },
    speed: 0.3,
    step_height: 1.0,
    turn: 30.0,
    stalk: Stalk {
        circle: 5.0,
        stop: 2.0,
        step: 30.0,
    },
    arena: Arena {
        wake: 24.0,
        leash: 24.0,
        reset: 40.0,
        bar: 48.0,
        reset_ticks: 100,
        stuck_ticks: 100,
        home_ticks: 100,
        fade_across: 3.0,
        fade_rise: 2.0,
        fade_swap: 10,
    },
    threat: 200,
    phases: &PHASES,
    moves: &MOVES,
    clips: Clips {
        idle: ClipDef {
            key: "idle",
            length: 40,
            poses: &[
                pose(0, Pose::Ready),
                pose(20, Pose::Breathe),
                pose(40, Pose::Ready),
            ],
            cues: &[],
            conjure: &[],
        },
        walk: ClipDef {
            key: "walk",
            length: 20,
            poses: &[pose(0, Pose::Ready), pose(20, Pose::Ready)],
            cues: &[],
            conjure: &[],
        },
        wake: ClipDef {
            key: "wake",
            length: 30,
            poses: &[
                pose(0, Pose::Kneel),
                pose(12, Pose::Kneel),
                pose(24, Pose::Rise),
                pose(30, Pose::Ready),
            ],
            cues: &[cue(0, MIST), burst(0, Burst::GoldenMist)],
            conjure: &[],
        },
        kneel: ClipDef {
            key: "kneel",
            length: 80,
            poses: &[
                pose(0, Pose::Ready),
                pose(6, Pose::Kneel),
                pose(60, Pose::Kneel),
                pose(80, Pose::Rise),
            ],
            cues: &[cue(0, STUNNED), burst(0, Burst::Crit)],
            conjure: &[],
        },
        fade: ClipDef {
            key: "fade",
            length: 20,
            poses: &[
                pose(0, Pose::Ready),
                pose(10, Pose::Vanish),
                pose(20, Pose::Ready),
            ],
            cues: &[
                cue(0, MIST),
                burst(0, Burst::GoldenMist),
                burst(10, Burst::GoldenMist),
            ],
            conjure: &[],
        },
        death: ClipDef {
            key: "death",
            length: 20,
            poses: &[
                pose(0, Pose::Ready),
                pose(8, Pose::Kneel),
                pose(20, Pose::Collapse),
            ],
            cues: &[
                cue(0, DEATH),
                burst(0, Burst::GoldenMist),
                cue(10, MIST),
                burst(10, Burst::GoldenMist),
            ],
            conjure: &[],
        },
    },
    daggers: Daggers {
        speed: 1.5,
        ticks: 40,
        damage: 4.0,
        harm: Harm::Holy,
        reaction: Reaction::Flinch,
        hand: DVec3::new(0.5, 2.2, 0.4),
    },
    experience: Experience {
        base: 100,
        per_fighter: 50,
    },
    drops: &[
        Loot {
            item: "minecraft:enchanted_golden_apple",
            count: 1,
            per_fighter: 0,
        },
        Loot {
            item: "minecraft:gold_block",
            count: 2,
            per_fighter: 0,
        },
        Loot {
            item: "minecraft:gold_ingot",
            count: 0,
            per_fighter: 3,
        },
    ],
    voices: Voices {
        ambient: Sound {
            event: "entity.evoker.ambient",
            volume: 1.0,
            pitch: 0.7,
        },
        hurt: Sound {
            event: "entity.illusioner.hurt",
            volume: 1.0,
            pitch: 0.7,
        },
        death: Sound {
            event: "entity.evoker.death",
            volume: 1.0,
            pitch: 0.6,
        },
        step: Sound {
            event: "entity.iron_golem.step",
            volume: 1.0,
            pitch: 0.7,
        },
    },
    cues: Cues {
        critical: &[
            sound("entity.player.attack.crit", 1.0, 1.0),
            Cue::Burst(Burst::Crit),
        ],
        dagger_throw: &[sound("item.trident.throw", 1.0, 1.0)],
        dagger_hit: &[sound("item.trident.hit", 1.0, 1.0)],
        felled: &[sound("ui.toast.challenge_complete", 1.0, 1.0)],
    },
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{IDS, check, def};

    fn with_move(key: &str, change: impl FnOnce(&mut MoveDef)) -> BossDef {
        let mut moves = MARGIT.moves.to_vec();
        let index = MARGIT.move_index(key).unwrap();
        change(&mut moves[index]);
        BossDef {
            moves: Box::leak(moves.into_boxed_slice()),
            ..MARGIT
        }
    }

    fn leak<T: Clone>(items: Vec<T>) -> &'static [T] {
        Box::leak(items.into_boxed_slice())
    }

    #[test]
    fn margit_passes_its_checks() {
        assert_eq!(check(&MARGIT), Ok(()));
        assert_eq!(MARGIT.source, DataSource::Estimate);
        assert_eq!(IDS, ["iw4l:margit"]);
        assert!(std::ptr::eq(def("iw4l:margit").unwrap(), &MARGIT));
        assert!(def("iw4l:morgott").is_none());
        assert_eq!(MARGIT.moves.len(), 8);
    }

    #[test]
    fn a_strike_past_the_end_of_its_move_is_refused() {
        let def = with_move("cane_slam", |mv| {
            let mut strikes = mv.strikes.to_vec();
            strikes[0].to = mv.length;
            mv.strikes = leak(strikes);
        });
        let refused = check(&def).unwrap_err();
        assert!(refused.contains("cane_slam"), "{refused}");
        assert!(refused.contains("past the end"), "{refused}");
    }

    #[test]
    fn every_distance_up_to_30_blocks_has_a_move_in_both_phases() {
        for phase in [1, 2] {
            for step in 0..=300 {
                let distance = f64::from(step) / 10.0;
                assert!(
                    MARGIT.moves.iter().any(|mv| mv.in_phase(phase)
                        && mv.pick.near <= distance
                        && distance <= mv.pick.far),
                    "phase {phase} at {distance}"
                );
            }
        }
        let dagger = with_move("dagger_throw", |mv| mv.pick.far = 20.0);
        assert!(check(&dagger).unwrap_err().contains("no move of phase 1"));
    }

    #[test]
    fn the_hammer_moves_are_phase_two_only() {
        let mut second: Vec<&str> = MARGIT
            .moves
            .iter()
            .filter(|mv| !mv.in_phase(1))
            .map(|mv| mv.key)
            .collect();
        second.sort();
        assert_eq!(second, ["hammer_leap", "hammer_slam"]);
        for mv in MARGIT.moves {
            let hammer = mv
                .conjure
                .iter()
                .any(|conjure| conjure.weapon == Weapon::Hammer);
            assert_eq!(hammer, mv.phases == [2], "{}", mv.key);
            assert!(mv.in_phase(2), "{}", mv.key);
        }
        assert!(MARGIT.phases[0].shift.is_none());
        let shift = MARGIT.phases[1].shift.unwrap();
        assert_eq!(shift.length, 30);
        assert!(
            shift
                .conjure
                .iter()
                .any(|conjure| conjure.weapon == Weapon::Hammer)
        );
    }

    #[test]
    fn a_strike_tracked_until_fewer_than_8_ticks_before_is_refused_unless_it_leaps() {
        let late = with_move("cane_slam", |mv| mv.track = leak(vec![window(0, 21)]));
        assert!(check(&late).unwrap_err().contains("less than 8 ticks"));
        let leap = with_move("leap_slam", |mv| mv.track = leak(vec![window(0, 29)]));
        assert_eq!(check(&leap), Ok(()));
    }

    #[test]
    fn a_branch_to_an_unknown_move_or_poses_out_of_order_are_refused() {
        let branch = with_move("cane_combo", |mv| {
            let mut branches = mv.branches.to_vec();
            branches[3].to = BranchTo::Move("cane_flurry");
            mv.branches = leak(branches);
        });
        assert!(check(&branch).unwrap_err().contains("cane_flurry"));
        let first = with_move("leap_slam", |mv| {
            let mut branches = mv.branches.to_vec();
            branches[0].at = 0;
            mv.branches = leak(branches);
        });
        assert!(check(&first).unwrap_err().contains("at its start"));
        let order = with_move("cane_slam", |mv| {
            let mut poses = mv.poses.to_vec();
            poses.swap(1, 2);
            mv.poses = leak(poses);
        });
        assert!(check(&order).unwrap_err().contains("out of order"));
        let short = with_move("cane_slam", |mv| {
            let mut poses = mv.poses.to_vec();
            poses.pop();
            mv.poses = leak(poses);
        });
        assert!(check(&short).unwrap_err().contains("not at the end"));
    }

    #[test]
    fn a_golden_weapon_not_shown_during_its_strike_is_refused() {
        let late = with_move("sword_sweep", |mv| {
            mv.conjure = leak(vec![Conjure {
                weapon: Weapon::Sword,
                from: 0,
                to: 24,
            }]);
        });
        assert!(check(&late).unwrap_err().contains("Sword"));
        let thrown = with_move("dagger_throw", |mv| mv.conjure = &[]);
        assert!(check(&thrown).unwrap_err().contains("no dagger"));
    }

    #[test]
    fn a_critical_lands_where_his_twenty_tick_rise_starts() {
        let kneel = MARGIT.clips.kneel;
        let rise = kneel.length - MARGIT.critical.recovery;
        assert_eq!(rise, 60);
        let keys: Vec<(u16, Pose)> = kneel.poses.iter().map(|key| (key.at, key.pose)).collect();
        assert_eq!(
            &keys[keys.len() - 2..],
            [(rise, Pose::Kneel), (kneel.length, Pose::Rise)]
        );
    }

    #[test]
    fn every_golden_weapon_forms_with_chime_and_beacon_and_every_slam_booms() {
        for mv in MARGIT.moves {
            let at = |tick: u16| -> Vec<Cue> {
                mv.cues
                    .iter()
                    .filter(|key| key.at == tick)
                    .map(|key| key.cue)
                    .collect()
            };
            for conjure in mv.conjure {
                let cues = at(conjure.from);
                assert!(cues.contains(&CHIME), "{}", mv.key);
                assert!(cues.contains(&BEACON), "{}", mv.key);
            }
            let slams: Vec<u16> = mv
                .cues
                .iter()
                .filter(|key| key.cue == Cue::Burst(Burst::Slam))
                .map(|key| key.at)
                .collect();
            for tick in slams {
                let cues = at(tick);
                assert!(cues.contains(&BOOM), "{}", mv.key);
                assert!(
                    cues.contains(&SMASH) || cues.contains(&SMASH_HEAVY),
                    "{}",
                    mv.key
                );
            }
        }
    }

    #[test]
    fn health_grows_with_the_fighters_at_the_wake() {
        let factors: Vec<f32> = (1..=5).map(|n| MARGIT.scaling.factor(n)).collect();
        assert_eq!(factors[..4], [1.0, 1.6, 2.2, 2.8]);
        assert!((factors[4] - 3.4).abs() < 1e-6);
        assert_eq!(MARGIT.scaling.factor(0), 1.0);
    }
}
