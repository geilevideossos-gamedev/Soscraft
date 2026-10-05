use playerstate_iw4::{PlayerState, UserCmd, pm_flags};

use crate::swim::input_push;
use crate::{
    CollisionBackend, JumpCheckContext, JumpCheckResult, MoveBounds, Pml, WATER_BLOCK,
    WalkMoveContext, jump, stance_surface_type, step_slide_move,
};

const TICK_MS: i32 = 50;
const TICK_SECONDS: f32 = 0.05;
const PER_TICK: f32 = WATER_BLOCK / TICK_SECONDS;
const WALK_SPEED: f32 = 0.1;
const SPRINT_SPEED: f32 = 0.13;
const AIR_SPEED: f32 = 0.02;
const SPRINT_AIR_SPEED: f32 = 0.026;
const SNEAK_SPEED: f32 = 0.3;
const JUMP_POWER: f32 = 0.42;
const SPRINT_JUMP_BOOST: f32 = 0.2;
const GRAVITY: f32 = 0.08;
const VERTICAL_DRAG: f32 = 0.98;
const FLUID_SPEED: f32 = 0.02;
const FLUID_RISE: f32 = 0.04;
const BUTTON_JUMP: u32 = 0x400;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stuck {
    pub multiplier: [f32; 3],
    pub fluid_sink: Option<f32>,
}

pub fn stuck_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &mut UserCmd,
    stuck: Stuck,
    context: WalkMoveContext,
    bounds: MoveBounds,
    collision: &C,
) {
    let frame_start = cmd.server_time.wrapping_sub(pml.msec);
    if pml.walking != 0 {
        if ps.pm_flags & pm_flags::JUMPING != 0 {
            jump::clear_state(ps);
        }
        let gate = JumpCheckContext {
            time_since_jump: cmd.server_time.wrapping_sub(ps.jump_time),
            old_buttons: context.old_buttons,
            stance_surface_type: stance_surface_type(ps) as u8,
        };
        if let JumpCheckResult::Launched { .. } = jump::check(ps, pml, cmd, gate, context.jump) {
            ps.jump_time = frame_start;
        }
    }
    let rising_ms = ps
        .jump_time
        .wrapping_add(TICK_MS)
        .wrapping_sub(frame_start)
        .clamp(0, pml.msec);
    let rise = rising_ms as f32 / pml.msec.max(1) as f32;
    let grounded = pml.walking != 0 || rising_ms > 0;
    let sprinting = ps.pm_flags & pm_flags::SPRINTING != 0;
    let speed = match (stuck.fluid_sink, grounded, sprinting) {
        (Some(_), _, _) => FLUID_SPEED,
        (None, true, false) => WALK_SPEED,
        (None, true, true) => SPRINT_SPEED,
        (None, false, false) => AIR_SPEED,
        (None, false, true) => SPRINT_AIR_SPEED,
    };
    let sneaking = ps.pm_flags & (pm_flags::CROUCH | pm_flags::PRONE) != 0;
    let push = input_push(
        [pml.forward[0], pml.forward[1]],
        [pml.right[0], pml.right[1]],
        cmd.forwardmove,
        cmd.rightmove,
        if sneaking { speed * SNEAK_SPEED } else { speed },
    );
    let mut step = [push[0], push[1], 0.0];
    let facing = libm::sqrtf(pml.forward[0] * pml.forward[0] + pml.forward[1] * pml.forward[1]);
    if sprinting && facing > 0.0 {
        step[0] += pml.forward[0] / facing * SPRINT_JUMP_BOOST * rise;
        step[1] += pml.forward[1] / facing * SPRINT_JUMP_BOOST * rise;
    }
    let lift = if cmd.buttons & BUTTON_JUMP != 0 {
        FLUID_RISE
    } else {
        0.0
    };
    let fall = match stuck.fluid_sink {
        _ if pml.walking != 0 && rising_ms == 0 => 0.0,
        Some(sink) => lift - sink,
        None => -GRAVITY * VERTICAL_DRAG,
    };
    step[2] = JUMP_POWER * rise + fall * (1.0 - rise);
    ps.velocity = [
        step[0] * stuck.multiplier[0] * PER_TICK,
        step[1] * stuck.multiplier[1] * PER_TICK,
        step[2] * stuck.multiplier[2] * PER_TICK,
    ];
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    ps.velocity[2] = ps.velocity[2].min(0.0);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AirMoveContext, CmdScaleWalkContext, GroundTraceInput, JumpLaunchContext};
    use playerstate_iw4::ENTITYNUM_NONE;

    const BUSH: [f32; 3] = [0.8, 0.8, 0.75];
    const WEB: [f32; 3] = [0.25, 0.25, 0.05];
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

    fn context() -> WalkMoveContext {
        let air = AirMoveContext {
            player_spectate_speed_scale: 1.0,
            shellshock_gravity_scale: 1.0,
            shellshock_gravity_bias: 0.0,
        };
        WalkMoveContext {
            cmd_scale: CmdScaleWalkContext {
                player_back_speed_scale: 0.7,
                player_strafe_speed_scale: 0.8,
                player_sprint_speed_scale: 1.5,
                player_last_stand_crawl_speed_scale: 0.15,
                weapon_move_speed_scale: 1.0,
                weapon_ads_move_speed_scale: 1.0,
                shellshock_affects_movement: false,
            },
            weapon_move_scale: 1.0,
            old_buttons: 0,
            jump: JumpLaunchContext {
                jump_height: 39.0,
                dive: false,
                crouch_jump_scale: 1.0,
                jump_ladder_push_vel: 128.0,
            },
            air,
        }
    }

    fn frame(msec: i32, walking: bool) -> Pml {
        Pml {
            forward: [1.0, 0.0, 0.0],
            right: [0.0, -1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            frametime: msec as f32 * 0.001,
            msec,
            walking: u32::from(walking),
            ground_plane: u32::from(walking),
            almost_ground_plane: u32::from(walking),
            ground_trace: [0; 11],
            previous_origin: [0.0; 3],
            previous_velocity: [0.0; 3],
            holdrand: 0,
            jump_animations: [None; 4],
            mantle_movetype: None,
            landing_animation: false,
        }
    }

    fn ahead(buttons: u32) -> UserCmd {
        UserCmd {
            forwardmove: 127,
            buttons,
            ..UserCmd::default()
        }
    }

    fn moved(
        ps: &mut PlayerState,
        cmd: UserCmd,
        multiplier: [f32; 3],
        frames: &[i32],
        walking: bool,
    ) -> [f32; 3] {
        let start = ps.origin;
        let mut time = cmd.server_time;
        for &msec in frames {
            time += msec;
            let mut cmd = UserCmd {
                server_time: time,
                ..cmd
            };
            stuck_move(
                ps,
                &mut frame(msec, walking),
                &mut cmd,
                Stuck {
                    multiplier,
                    fluid_sink: None,
                },
                context(),
                BOUNDS,
                &Open,
            );
        }
        core::array::from_fn(|k| (ps.origin[k] - start[k]) / WATER_BLOCK)
    }

    fn standing() -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.jump_time = -10_000;
        ps
    }

    fn airborne() -> PlayerState {
        let mut ps = standing();
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps
    }

    #[test]
    fn a_tick_in_a_bush_or_web_moves_the_vanilla_stuck_step() {
        let cases = [
            (BUSH, 0, 0.098 * 0.8),
            (BUSH, pm_flags::SPRINTING, 0.13 * 0.98 * 0.8),
            (BUSH, pm_flags::CROUCH, 0.098 * 0.3 * 0.8),
            (WEB, 0, 0.098 * 0.25),
        ];
        for (multiplier, flags, step) in cases {
            for frames in [&[50][..], &[10, 15, 25], &[5; 10]] {
                let mut ps = standing();
                ps.pm_flags = flags;
                let walked = moved(&mut ps, ahead(0), multiplier, frames, true);
                assert!(
                    (walked[0] - step).abs() < 1e-5,
                    "{flags:x} {frames:?} {walked:?}"
                );
                assert!(walked[2].abs() < 1e-6, "{walked:?}");
            }
        }
        let mut ps = airborne();
        let drifted = moved(&mut ps, ahead(0), BUSH, &[50], false);
        assert!((drifted[0] - 0.02 * 0.98 * 0.8).abs() < 1e-6, "{drifted:?}");
    }

    #[test]
    fn momentum_is_dropped_and_falls_are_slowed() {
        for (multiplier, fall) in [(BUSH, 0.0784 * 0.75), (WEB, 0.0784 * 0.05)] {
            let mut ps = airborne();
            ps.velocity = [600.0, -300.0, -900.0];
            let dropped = moved(&mut ps, UserCmd::default(), multiplier, &[50], false);
            assert!(
                dropped[0].abs() < 1e-6 && dropped[1].abs() < 1e-6,
                "{dropped:?}"
            );
            assert!((dropped[2] + fall).abs() < 1e-5, "{dropped:?}");
            assert!(
                ps.velocity[2] <= 0.0 && ps.velocity[2] > -60.0,
                "{:?}",
                ps.velocity
            );
        }
    }

    #[test]
    fn a_bush_or_web_in_water_or_lava_scales_the_fluid_step() {
        let swim = |ps: &mut PlayerState, cmd: UserCmd, multiplier, sink| {
            let start = ps.origin;
            let mut cmd = UserCmd {
                server_time: 1050,
                ..cmd
            };
            stuck_move(
                ps,
                &mut frame(50, false),
                &mut cmd,
                Stuck {
                    multiplier,
                    fluid_sink: Some(sink),
                },
                context(),
                BOUNDS,
                &Open,
            );
            core::array::from_fn::<f32, 3, _>(|k| (ps.origin[k] - start[k]) / WATER_BLOCK)
        };
        let waded = swim(&mut airborne(), ahead(0), BUSH, 0.005);
        assert!((waded[0] - 0.02 * 0.98 * 0.8).abs() < 1e-6, "{waded:?}");
        assert!((waded[2] + 0.005 * 0.75).abs() < 1e-6, "{waded:?}");
        let risen = swim(&mut airborne(), ahead(BUTTON_JUMP), BUSH, 0.005);
        assert!((risen[2] - 0.035 * 0.75).abs() < 1e-6, "{risen:?}");
        let sunk = swim(&mut airborne(), UserCmd::default(), WEB, 0.02);
        assert!((sunk[2] + 0.02 * 0.05).abs() < 1e-6, "{sunk:?}");
    }

    #[test]
    fn a_jump_in_a_bush_rises_the_scaled_jump_in_one_tick_whatever_the_frames() {
        for frames in [&[60][..], &[20, 20, 20], &[8; 7]] {
            let mut ps = standing();
            let total: i32 = frames.iter().sum();
            let mut start = ahead(BUTTON_JUMP);
            start.server_time = 1000;
            let first = moved(&mut ps, start, BUSH, &frames[..1], true);
            assert_eq!(ps.jump_time, 1000, "the rise starts with the frame");
            assert_eq!(ps.ground_entity_num, ENTITYNUM_NONE);
            let rest = UserCmd {
                server_time: 1000 + frames[0],
                ..ahead(BUTTON_JUMP)
            };
            let later = moved(&mut ps, rest, BUSH, &frames[1..], false);
            let fall_ms = (total - 50) as f32;
            let risen = first[2] + later[2];
            let expected = 0.42 * 0.75 - 0.0784 * 0.75 * fall_ms / 50.0;
            assert!((risen - expected).abs() < 1e-5, "{frames:?} {risen}");
            assert!(ps.velocity[2] <= 0.0, "{:?}", ps.velocity);
        }
        let mut held = standing();
        let mut cmd = ahead(BUTTON_JUMP);
        cmd.server_time = 1000;
        let mut repeat = context();
        repeat.old_buttons = BUTTON_JUMP;
        let mut pml = frame(50, true);
        let mut sent = UserCmd {
            server_time: 1050,
            ..cmd
        };
        let bush = Stuck {
            multiplier: BUSH,
            fluid_sink: None,
        };
        stuck_move(&mut held, &mut pml, &mut sent, bush, repeat, BOUNDS, &Open);
        assert!(
            held.origin[2].abs() < 1e-6,
            "a held jump waits for a new press"
        );
    }
}
