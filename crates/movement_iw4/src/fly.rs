use playerstate_iw4::{ENTITYNUM_NONE, PlayerState, UserCmd, buttons, pm_flags};

use crate::swim::input_push;
use crate::{
    CollisionBackend, MoveBounds, Pml, WATER_BLOCK, complete_ground_trace, settle, step_slide_move,
};

pub const TOGGLE_WINDOW_MS: i32 = 350;
const TICK_SECONDS: f32 = 0.05;
const PER_TICK: f32 = WATER_BLOCK / TICK_SECONDS;
const FLYING_SPEED: f32 = 0.05;
const SPRINT_SCALE: f32 = 2.0;
const VERTICAL_SCALE: f32 = 3.0;
const AIR_DRAG: f32 = 0.91;
const VERTICAL_DRAG: f32 = 0.6;
const JUMP_POWER: f32 = 0.42;
const SPRINT_JUMP_BOOST: f32 = 0.2;
const HELD_STANCE: u32 = buttons::JUMP | buttons::CROUCH | buttons::PRONE | buttons::STANCE_HELD;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlyTap {
    first: Option<i32>,
    was_down: bool,
    pending: bool,
}

impl FlyTap {
    pub fn press(&mut self, jump_down: bool, now_ms: i32) -> bool {
        if self
            .first
            .is_some_and(|first| now_ms.wrapping_sub(first) >= TOGGLE_WINDOW_MS)
        {
            self.first = None;
        }
        if jump_down && !self.was_down {
            if self.first.take().is_some() {
                self.pending = true;
            } else {
                self.first = Some(now_ms);
            }
        }
        self.was_down = jump_down;
        self.pending
    }

    pub fn consume(&mut self) {
        self.pending = false;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Flight {
    pub up: bool,
    pub down: bool,
    pub sprint: bool,
    pub takeoff: bool,
    pub noclip: bool,
}

pub fn prepare(ps: &mut PlayerState, cmd: &mut UserCmd, swimming: bool) -> Option<Flight> {
    let toggled = cmd.buttons & buttons::MC_FLY_TOGGLE != 0;
    cmd.buttons &= !buttons::MC_FLY_TOGGLE;
    if ps.pm_flags & pm_flags::MC_MAY_FLY == 0 {
        ps.pm_flags &= !pm_flags::MC_FLYING;
        return None;
    }
    if ps.pm_flags & pm_flags::MANTLE != 0 {
        return None;
    }
    let noclip = ps.pm_flags & pm_flags::MC_NOCLIP != 0;
    let was_flying = ps.pm_flags & pm_flags::MC_FLYING != 0;
    let mut takeoff = false;
    if noclip {
        ps.pm_flags |= pm_flags::MC_FLYING;
    } else if toggled && (was_flying || !swimming) {
        ps.pm_flags ^= pm_flags::MC_FLYING;
        takeoff = !was_flying && ps.ground_entity_num != ENTITYNUM_NONE;
    }
    if ps.pm_flags & pm_flags::MC_FLYING == 0 {
        return None;
    }
    ps.pm_flags &= !pm_flags::LADDER;
    let held = cmd.buttons;
    cmd.buttons &= !HELD_STANCE;
    Some(Flight {
        up: held & buttons::JUMP != 0,
        down: held & (buttons::CROUCH | buttons::PRONE) != 0,
        sprint: held & buttons::SPRINT != 0 && cmd.forwardmove > 0,
        takeoff,
        noclip,
    })
}

pub fn fly_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    flight: Flight,
    stuck: Option<[f32; 3]>,
    bounds: MoveBounds,
    collision: &C,
) {
    let ticks = pml.frametime / TICK_SECONDS;
    let speed = if flight.sprint {
        FLYING_SPEED * SPRINT_SCALE
    } else {
        FLYING_SPEED
    };
    let push = input_push(
        [pml.forward[0], pml.forward[1]],
        [pml.right[0], pml.right[1]],
        cmd.forwardmove,
        cmd.rightmove,
        speed,
    );
    let lift = (f32::from(u8::from(flight.up)) - f32::from(u8::from(flight.down)))
        * FLYING_SPEED
        * VERTICAL_SCALE;
    let old = ps.velocity.map(|speed| speed / PER_TICK);
    let rise = lift / (1.0 - VERTICAL_DRAG);
    let mut v = match stuck {
        Some(_) => [push[0], push[1], settle(old[2], rise, VERTICAL_DRAG, ticks)],
        None => [
            settle(old[0], push[0] / (1.0 - AIR_DRAG), AIR_DRAG, ticks),
            settle(old[1], push[1] / (1.0 - AIR_DRAG), AIR_DRAG, ticks),
            settle(old[2], rise, VERTICAL_DRAG, ticks),
        ],
    };
    if flight.takeoff {
        v[2] = rise + impulse(JUMP_POWER + lift - rise, VERTICAL_DRAG, ticks);
        let facing = libm::sqrtf(pml.forward[0] * pml.forward[0] + pml.forward[1] * pml.forward[1]);
        if flight.sprint && facing > 0.0 {
            let boost = impulse(SPRINT_JUMP_BOOST, AIR_DRAG, ticks);
            v[0] += pml.forward[0] / facing * boost;
            v[1] += pml.forward[1] / facing * boost;
        }
        ps.ground_entity_num = ENTITYNUM_NONE;
        pml.walking = 0;
        pml.ground_plane = 0;
        pml.almost_ground_plane = 0;
    }
    let multiplier = stuck.unwrap_or([1.0; 3]);
    ps.velocity = core::array::from_fn(|k| v[k] * PER_TICK * multiplier[k]);
    let requested_z = ps.velocity[2];
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    if multiplier[2] > 0.0 {
        ps.velocity[2] /= multiplier[2];
    }
    complete_ground_trace(ps, pml, bounds, cmd.forwardmove, collision);
    if requested_z < 0.0 && !flight.noclip && ps.ground_entity_num != ENTITYNUM_NONE {
        ps.pm_flags &= !pm_flags::MC_FLYING;
    }
}

fn impulse(value: f32, keep: f32, ticks: f32) -> f32 {
    if ticks <= 0.0 {
        return value;
    }
    value * (1.0 - libm::powf(keep, ticks)) / ((1.0 - keep) * ticks)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GroundTraceInput;

    struct Open;

    impl CollisionBackend for Open {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            trace_iw4::Trace {
                fraction: 1.0,
                endpos: input.end,
                ..trace_iw4::Trace::default()
            }
        }
    }

    const BOUNDS: MoveBounds = MoveBounds {
        mins: [-10.8, -10.8, 0.0],
        maxs: [10.8, 10.8, 64.8],
        tracemask: 0,
        heights: [64.8, 54.0, 30.0],
    };

    fn pml(msec: i32) -> Pml {
        Pml {
            forward: [1.0, 0.0, 0.0],
            right: [0.0, -1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            frametime: msec as f32 * 0.001,
            msec,
            walking: 0,
            ground_plane: 0,
            almost_ground_plane: 0,
            ground_trace: [0; 11],
            previous_origin: [0.0; 3],
            previous_velocity: [0.0; 3],
            holdrand: 0,
            jump_animations: [None; 4],
            mantle_movetype: None,
            landing_animation: false,
        }
    }

    fn flier() -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps.pm_flags |= pm_flags::MC_MAY_FLY | pm_flags::MC_FLYING;
        ps
    }

    fn fly(ps: &mut PlayerState, flight: Flight, forwardmove: i8, msec: i32, frames: u32) {
        let cmd = UserCmd {
            forwardmove,
            ..UserCmd::default()
        };
        for _ in 0..frames {
            fly_move(ps, &mut pml(msec), &cmd, flight, None, BOUNDS, &Open);
        }
    }

    fn blocks_per_tick(units_per_second: f32) -> f32 {
        units_per_second / PER_TICK
    }

    #[test]
    fn a_second_press_within_seven_ticks_toggles_flight() {
        let mut tap = FlyTap::default();
        assert!(!tap.press(true, 1000));
        assert!(!tap.press(false, 1100));
        assert!(tap.press(true, 1300));
        assert!(tap.press(true, 1350));
        tap.consume();
        assert!(!tap.press(false, 1400));
        assert!(!tap.press(true, 1450));
        assert!(!tap.press(false, 1500));
        assert!(tap.press(true, 1600));

        let mut slow = FlyTap::default();
        assert!(!slow.press(true, 1000));
        assert!(!slow.press(false, 1100));
        assert!(!slow.press(true, 1400));
        assert!(!slow.press(false, 1500));
        assert!(slow.press(true, 1700));

        let mut held = FlyTap::default();
        for now in (1000..3000).step_by(16) {
            assert!(!held.press(true, now));
        }
    }

    #[test]
    fn flight_reaches_vanilla_speeds_at_any_frame_rate() {
        for msec in [50, 8] {
            let mut ps = flier();
            fly(&mut ps, Flight::default(), 127, msec, 1);
            if msec == 50 {
                assert!(
                    (ps.origin[0] - 0.05 * 0.98 * WATER_BLOCK).abs() < 1e-3,
                    "{:?}",
                    ps.origin
                );
            }
            fly(&mut ps, Flight::default(), 127, msec, 6000 / msec as u32);
            let cruise = blocks_per_tick(ps.velocity[0]);
            assert!(
                (cruise - 0.049 / 0.09).abs() < 1e-3,
                "msec {msec}: {cruise}"
            );
            assert!(
                (ps.velocity[0] - 392.0).abs() < 1.0,
                "msec {msec}: {:?}",
                ps.velocity
            );
            assert_eq!(ps.velocity[2], 0.0);

            let mut sprinter = flier();
            let sprint = Flight {
                sprint: true,
                ..Flight::default()
            };
            fly(&mut sprinter, sprint, 127, msec, 6000 / msec as u32);
            assert!(
                (sprinter.velocity[0] / ps.velocity[0] - 2.0).abs() < 1e-3,
                "msec {msec}: {:?} {:?}",
                sprinter.velocity,
                ps.velocity
            );
        }
    }

    #[test]
    fn jump_rises_and_sneak_sinks_like_vanilla() {
        let up = Flight {
            up: true,
            ..Flight::default()
        };
        let mut ps = flier();
        fly(&mut ps, up, 0, 50, 1);
        assert!(
            (ps.origin[2] / WATER_BLOCK - 0.15).abs() < 1e-5,
            "{:?}",
            ps.origin
        );
        let first = ps.origin[2];
        fly(&mut ps, Flight::default(), 0, 50, 1);
        assert!(
            ((ps.origin[2] - first) / WATER_BLOCK - 0.09).abs() < 1e-5,
            "{:?}",
            ps.origin
        );
        for msec in [50, 8] {
            let mut riser = flier();
            fly(&mut riser, up, 0, msec, 3000 / msec as u32);
            let rise = blocks_per_tick(riser.velocity[2]);
            assert!((rise - 0.375).abs() < 1e-3, "msec {msec}: {rise}");
            let down = Flight {
                down: true,
                ..Flight::default()
            };
            let mut sinker = flier();
            fly(&mut sinker, down, 0, msec, 3000 / msec as u32);
            assert!(
                (blocks_per_tick(sinker.velocity[2]) + 0.375).abs() < 1e-3,
                "msec {msec}"
            );
            assert_ne!(sinker.pm_flags & pm_flags::MC_FLYING, 0);
        }
    }

    #[test]
    fn taking_off_from_the_ground_jumps_into_the_air() {
        let mut ps = flier();
        ps.ground_entity_num = 0;
        let takeoff = Flight {
            takeoff: true,
            ..Flight::default()
        };
        let mut frame = pml(50);
        frame.walking = 1;
        let cmd = UserCmd::default();
        fly_move(&mut ps, &mut frame, &cmd, takeoff, None, BOUNDS, &Open);
        assert!(
            (ps.origin[2] / WATER_BLOCK - JUMP_POWER).abs() < 1e-5,
            "{:?}",
            ps.origin
        );
        assert_eq!((ps.ground_entity_num, frame.walking), (ENTITYNUM_NONE, 0));

        for msec in [50, 8] {
            let mut ps = flier();
            ps.ground_entity_num = 0;
            fly_move(&mut ps, &mut pml(msec), &cmd, takeoff, None, BOUNDS, &Open);
            fly(&mut ps, Flight::default(), 0, msec, 3000 / msec as u32);
            let peak = ps.origin[2] / WATER_BLOCK;
            assert!(
                (peak - JUMP_POWER / (1.0 - VERTICAL_DRAG)).abs() < 1e-3,
                "msec {msec}: {peak}"
            );
        }
    }

    #[test]
    fn cobwebs_slow_flight_without_momentum() {
        let web = [0.25, 0.25, 0.05];
        let cmd = UserCmd {
            forwardmove: 127,
            ..UserCmd::default()
        };
        for msec in [50, 8] {
            let mut ps = flier();
            for _ in 0..1000 / msec {
                fly_move(
                    &mut ps,
                    &mut pml(msec),
                    &cmd,
                    Flight::default(),
                    Some(web),
                    BOUNDS,
                    &Open,
                );
            }
            let moved = ps.origin[0] / WATER_BLOCK;
            assert!(
                (moved - 20.0 * 0.049 * 0.25).abs() < 1e-3,
                "msec {msec}: {moved}"
            );
        }

        let up = Flight {
            up: true,
            ..Flight::default()
        };
        for msec in [50, 8] {
            let mut ps = flier();
            let climb = |frames: i32, ps: &mut PlayerState| {
                for _ in 0..frames {
                    fly_move(
                        ps,
                        &mut pml(msec),
                        &UserCmd::default(),
                        up,
                        Some(web),
                        BOUNDS,
                        &Open,
                    );
                }
            };
            climb(3000 / msec, &mut ps);
            let start = ps.origin[2];
            climb(1000 / msec, &mut ps);
            let risen = (ps.origin[2] - start) / WATER_BLOCK;
            assert!(
                (risen - 20.0 * 0.375 * 0.05).abs() < 1e-3,
                "msec {msec}: {risen}"
            );
        }
    }

    #[test]
    fn only_a_player_who_may_fly_toggles_flight() {
        let toggle = UserCmd {
            buttons: buttons::MC_FLY_TOGGLE | buttons::JUMP,
            ..UserCmd::default()
        };
        let mut walker = PlayerState::ZERO;
        walker.pm_flags |= pm_flags::MC_FLYING;
        let mut cmd = toggle;
        assert_eq!(prepare(&mut walker, &mut cmd, false), None);
        assert_eq!(walker.pm_flags & pm_flags::MC_FLYING, 0);
        assert_eq!(cmd.buttons, buttons::JUMP);

        let mut creative = PlayerState::ZERO;
        creative.pm_flags |= pm_flags::MC_MAY_FLY;
        let mut cmd = toggle;
        let flight = prepare(&mut creative, &mut cmd, false).expect("flying");
        assert!(flight.takeoff && flight.up && !flight.noclip);
        assert_ne!(creative.pm_flags & pm_flags::MC_FLYING, 0);
        assert_eq!(cmd.buttons, 0);
        let mut cmd = toggle;
        assert_eq!(prepare(&mut creative, &mut cmd, false), None);
        assert_eq!(creative.pm_flags & pm_flags::MC_FLYING, 0);
        assert_eq!(cmd.buttons, buttons::JUMP);

        let mut swimmer = PlayerState::ZERO;
        swimmer.pm_flags |= pm_flags::MC_MAY_FLY;
        let mut cmd = toggle;
        assert_eq!(prepare(&mut swimmer, &mut cmd, true), None);
        let mut mantler = PlayerState::ZERO;
        mantler.pm_flags |= pm_flags::MC_MAY_FLY | pm_flags::MANTLE;
        let mut cmd = toggle;
        assert_eq!(prepare(&mut mantler, &mut cmd, false), None);

        let mut spectator = PlayerState::ZERO;
        spectator.pm_flags |= pm_flags::MC_MAY_FLY | pm_flags::MC_NOCLIP;
        let mut cmd = toggle;
        let flight = prepare(&mut spectator, &mut cmd, false).expect("flying");
        assert!(flight.noclip && !flight.takeoff);
        assert_ne!(spectator.pm_flags & pm_flags::MC_FLYING, 0);

        let mut mantling_spectator = PlayerState::ZERO;
        mantling_spectator.pm_flags |=
            pm_flags::MC_MAY_FLY | pm_flags::MC_NOCLIP | pm_flags::MC_FLYING | pm_flags::MANTLE;
        let mut cmd = toggle;
        assert_eq!(prepare(&mut mantling_spectator, &mut cmd, false), None);
        mantling_spectator.pm_flags &= !pm_flags::MANTLE;
        let mut cmd = toggle;
        assert!(prepare(&mut mantling_spectator, &mut cmd, false).is_some());
    }

    #[test]
    fn flying_stands_up_and_reads_the_stance_keys_as_up_and_down() {
        let mut ps = flier();
        let mut cmd = UserCmd {
            buttons: buttons::CROUCH | buttons::STANCE_HELD | buttons::SPRINT | buttons::ATTACK,
            forwardmove: 127,
            ..UserCmd::default()
        };
        let flight = prepare(&mut ps, &mut cmd, false).expect("flying");
        assert_eq!(
            flight,
            Flight {
                down: true,
                sprint: true,
                ..Flight::default()
            }
        );
        assert_eq!(cmd.buttons, buttons::SPRINT | buttons::ATTACK);
        let mut prone = UserCmd {
            buttons: buttons::PRONE | buttons::SPRINT,
            rightmove: 127,
            ..UserCmd::default()
        };
        let flight = prepare(&mut ps, &mut prone, false).expect("flying");
        assert!(flight.down && !flight.sprint);
    }

    #[test]
    fn landing_ends_flight_but_hovering_does_not() {
        struct Floor;

        impl CollisionBackend for Floor {
            fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
                let floor = 0.0;
                if input.end[2] >= floor {
                    return trace_iw4::Trace {
                        fraction: 1.0,
                        endpos: input.end,
                        ..trace_iw4::Trace::default()
                    };
                }
                let travel = input.start[2] - input.end[2];
                let fraction = ((input.start[2] - floor) / travel).clamp(0.0, 1.0);
                trace_iw4::Trace {
                    fraction,
                    endpos: core::array::from_fn(|k| {
                        input.start[k] + (input.end[k] - input.start[k]) * fraction
                    }),
                    normal: [0.0, 0.0, 1.0],
                    hit_type: trace_iw4::HITTYPE_ENTITY,
                    hit_id: trace_iw4::ENTITYNUM_WORLD,
                    walkable: 1,
                    ..trace_iw4::Trace::default()
                }
            }
        }

        let cmd = UserCmd::default();
        let mut hover = flier();
        hover.origin[2] = 0.1;
        for _ in 0..20 {
            fly_move(
                &mut hover,
                &mut pml(50),
                &cmd,
                Flight::default(),
                None,
                BOUNDS,
                &Floor,
            );
        }
        assert_ne!(hover.pm_flags & pm_flags::MC_FLYING, 0);

        let down = Flight {
            down: true,
            ..Flight::default()
        };
        let mut lander = flier();
        lander.origin[2] = 30.0;
        for _ in 0..40 {
            fly_move(&mut lander, &mut pml(50), &cmd, down, None, BOUNDS, &Floor);
        }
        assert_eq!(lander.pm_flags & pm_flags::MC_FLYING, 0);
        assert_ne!(lander.ground_entity_num, ENTITYNUM_NONE);

        let mut spectator = flier();
        spectator.origin[2] = 30.0;
        let ghost = Flight {
            noclip: true,
            ..down
        };
        for _ in 0..40 {
            fly_move(
                &mut spectator,
                &mut pml(50),
                &cmd,
                ghost,
                None,
                BOUNDS,
                &Floor,
            );
        }
        assert_ne!(spectator.pm_flags & pm_flags::MC_FLYING, 0);
    }
}
