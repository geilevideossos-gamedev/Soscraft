use playerstate_iw4::{ENTITYNUM_NONE, PlayerState, UserCmd, buttons, pm_flags};

use crate::{
    CollisionBackend, MoveBounds, Pml, WATER_BLOCK, complete_ground_trace, step_slide_move,
};

const TICK_SECONDS: f64 = 0.05;
const PER_TICK: f64 = WATER_BLOCK as f64 / TICK_SECONDS;
const GRAVITY: f64 = 0.08;
const DRAG: [f64; 3] = [
    0.990_000_009_536_743_2,
    0.990_000_009_536_743_2,
    0.980_000_019_073_486_3,
];
const HELD: u32 =
    buttons::JUMP | buttons::CROUCH | buttons::PRONE | buttons::STANCE_HELD | buttons::SPRINT;
const BLOCKED: u32 = pm_flags::MC_FLYING | pm_flags::MANTLE | pm_flags::LADDER;

pub fn prepare(
    ps: &mut PlayerState,
    cmd: &mut UserCmd,
    old_buttons: u32,
    liquid: bool,
    climbing: bool,
) -> bool {
    let may = ps.pm_flags & pm_flags::MC_MAY_GLIDE != 0
        && ps.pm_flags & BLOCKED == 0
        && ps.ground_entity_num == ENTITYNUM_NONE
        && !liquid
        && !climbing;
    let fresh_jump = cmd.buttons & buttons::JUMP != 0 && old_buttons & buttons::JUMP == 0;
    if !may {
        ps.pm_flags &= !pm_flags::MC_GLIDING;
    } else if fresh_jump {
        ps.pm_flags |= pm_flags::MC_GLIDING;
    }
    if ps.pm_flags & pm_flags::MC_GLIDING == 0 {
        return false;
    }
    cmd.buttons &= !HELD;
    true
}

pub fn glide_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    stuck: Option<[f32; 3]>,
    bounds: MoveBounds,
    collision: &C,
) {
    let ticks = f64::from(pml.frametime) / TICK_SECONDS;
    let look = pml.forward.map(f64::from);
    let mut motion = ps.velocity.map(|speed| f64::from(speed) / PER_TICK);
    let moved = match stuck {
        Some(multiplier) => {
            let step = glide(motion, look, 1.0);
            motion = [0.0; 3];
            core::array::from_fn(|k| step[k] * f64::from(multiplier[k]))
        }
        None => {
            motion = glide(motion, look, ticks);
            motion
        }
    };
    ps.velocity = moved.map(|speed| (speed * PER_TICK) as f32);
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    if stuck.is_some() {
        ps.velocity = motion.map(|speed| (speed * PER_TICK) as f32);
    }
    complete_ground_trace(ps, pml, bounds, cmd.forwardmove, collision);
    if ps.ground_entity_num != ENTITYNUM_NONE {
        ps.pm_flags &= !pm_flags::MC_GLIDING;
    }
}

#[must_use]
pub fn glide(motion: [f64; 3], look: [f64; 3], ticks: f64) -> [f64; 3] {
    let pushed = glide_push(motion, look);
    core::array::from_fn(|k| {
        let keep = libm::pow(DRAG[k], ticks);
        keep * motion[k] + (pushed[k] - motion[k]) * DRAG[k] * (1.0 - keep) / (1.0 - DRAG[k])
    })
}

fn glide_push(motion: [f64; 3], look: [f64; 3]) -> [f64; 3] {
    let look_h = libm::sqrt(look[0] * look[0] + look[1] * look[1]);
    let speed_h = libm::sqrt(motion[0] * motion[0] + motion[1] * motion[1]);
    let lift = look_h * look_h;
    let mut m = motion;
    m[2] += GRAVITY * (-1.0 + lift * 0.75);
    if m[2] < 0.0 && look_h > 0.0 {
        let d = m[2] * -0.1 * lift;
        m[0] += look[0] * d / look_h;
        m[1] += look[1] * d / look_h;
        m[2] += d;
    }
    if look[2] > 0.0 && look_h > 0.0 {
        let d = speed_h * look[2] * 0.04;
        m[0] -= look[0] * d / look_h;
        m[1] -= look[1] * d / look_h;
        m[2] += d * 3.2;
    }
    if look_h > 0.0 {
        m[0] += (look[0] / look_h * speed_h - m[0]) * 0.1;
        m[1] += (look[1] / look_h * speed_h - m[1]) * 0.1;
    }
    m
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

    struct Floor;

    impl CollisionBackend for Floor {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            if input.end[2] >= 0.0 {
                return trace_iw4::Trace {
                    fraction: 1.0,
                    endpos: input.end,
                    ..trace_iw4::Trace::default()
                };
            }
            let travel = input.start[2] - input.end[2];
            let fraction = (input.start[2] / travel).clamp(0.0, 1.0);
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

    struct Wall(f32);

    impl CollisionBackend for Wall {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            let limit = self.0 - input.maxs[0];
            if input.end[0] <= limit {
                return trace_iw4::Trace {
                    fraction: 1.0,
                    endpos: input.end,
                    ..trace_iw4::Trace::default()
                };
            }
            let travel = input.end[0] - input.start[0];
            let fraction = if travel > 0.0 {
                ((limit - input.start[0]) / travel).clamp(0.0, 1.0)
            } else {
                0.0
            };
            trace_iw4::Trace {
                fraction,
                endpos: core::array::from_fn(|k| {
                    input.start[k] + (input.end[k] - input.start[k]) * fraction
                }),
                normal: [-1.0, 0.0, 0.0],
                hit_type: trace_iw4::HITTYPE_ENTITY,
                hit_id: trace_iw4::ENTITYNUM_WORLD,
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

    fn pml(msec: i32, look: [f32; 3]) -> Pml {
        Pml {
            forward: look,
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

    fn glider(height: f32) -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps.pm_flags |= pm_flags::MC_MAY_GLIDE | pm_flags::MC_GLIDING;
        ps.origin[2] = height;
        ps
    }

    fn per_tick(units_per_second: [f32; 3]) -> [f64; 3] {
        units_per_second.map(|speed| f64::from(speed) / PER_TICK)
    }

    fn close(a: [f64; 3], b: [f64; 3], epsilon: f64) -> bool {
        a.iter().zip(b).all(|(a, b)| (a - b).abs() < epsilon)
    }

    fn pitched(degrees: f64) -> [f64; 3] {
        let pitch = degrees.to_radians();
        [libm::cos(pitch), 0.0, -libm::sin(pitch)]
    }

    fn glide_frames<C: CollisionBackend>(
        ps: &mut PlayerState,
        msec: i32,
        frames: i32,
        look: [f32; 3],
        stuck: Option<[f32; 3]>,
        collision: &C,
    ) {
        for _ in 0..frames {
            glide_move(
                ps,
                &mut pml(msec, look),
                &UserCmd::default(),
                stuck,
                BOUNDS,
                collision,
            );
        }
    }

    #[test]
    fn one_tick_matches_hand_worked_vanilla_numbers() {
        let level = glide([1.0, 0.0, 0.0], [1.0, 0.0, 0.0], 1.0);
        assert!(
            close(
                level,
                [0.991_782_009_553_909_3, 0.0, -0.017_640_000_343_322_755],
                1e-12
            ),
            "{level:?}"
        );
        let climbing = glide([1.5, 0.0, 0.0], pitched(-30.0), 1.0);
        assert!(
            close(
                climbing,
                [1.460_608_889_070_153_4, 0.0, 0.062_352_501_213_550_57],
                1e-12
            ),
            "{climbing:?}"
        );
        let turned = glide([0.0, 1.0, 0.0], [1.0, 0.0, 0.0], 1.0);
        assert!(turned[0] > 0.09 && turned[1] < 0.9, "{turned:?}");
    }

    #[test]
    fn gliding_from_rest_covers_the_vanilla_path_at_any_frame_rate() {
        let mut vanilla = [0.0; 3];
        let mut path = [0.0; 3];
        for _ in 0..40 {
            vanilla = glide(vanilla, [1.0, 0.0, 0.0], 1.0);
            path = core::array::from_fn(|k| path[k] + vanilla[k]);
        }
        assert!(
            close(
                path,
                [7.789_822_986_994_259, 0.0, -4.869_636_999_177_878],
                1e-9
            ),
            "{path:?}"
        );
        for msec in [50, 8] {
            let top = 1000.0 * WATER_BLOCK;
            let mut ps = glider(top);
            glide_frames(&mut ps, msec, 2000 / msec, [1.0, 0.0, 0.0], None, &Open);
            let moved = [
                f64::from(ps.origin[0] / WATER_BLOCK),
                0.0,
                f64::from((ps.origin[2] - top) / WATER_BLOCK),
            ];
            assert!(close(moved, path, 0.3), "msec {msec}: {moved:?}");
            assert!(close(per_tick(ps.velocity), vanilla, 0.01), "msec {msec}");
            assert_ne!(ps.pm_flags & pm_flags::MC_GLIDING, 0);
        }
    }

    #[test]
    fn a_fresh_jump_in_the_air_starts_gliding_only_with_a_usable_elytra() {
        let jump = UserCmd {
            buttons: buttons::JUMP | buttons::SPRINT | buttons::ATTACK,
            ..UserCmd::default()
        };
        let mut ps = PlayerState::ZERO;
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps.pm_flags |= pm_flags::MC_MAY_GLIDE;
        let mut held = jump;
        assert!(!prepare(&mut ps, &mut held, buttons::JUMP, false, false));
        assert_eq!(held.buttons, jump.buttons);
        let mut pressed = jump;
        assert!(prepare(&mut ps, &mut pressed, 0, false, false));
        assert_eq!(pressed.buttons, buttons::ATTACK);
        assert_ne!(ps.pm_flags & pm_flags::MC_GLIDING, 0);
        let mut later = UserCmd::default();
        assert!(prepare(&mut ps, &mut later, buttons::JUMP, false, false));

        let mut grounded = PlayerState::ZERO;
        grounded.pm_flags |= pm_flags::MC_MAY_GLIDE;
        let mut cmd = jump;
        assert!(!prepare(&mut grounded, &mut cmd, 0, false, false));
        let mut bare = PlayerState::ZERO;
        bare.ground_entity_num = ENTITYNUM_NONE;
        let mut cmd = jump;
        assert!(!prepare(&mut bare, &mut cmd, 0, false, false));
        assert_eq!(bare.pm_flags & pm_flags::MC_GLIDING, 0);
        for (liquid, climbing) in [(true, false), (false, true)] {
            let mut ps = PlayerState::ZERO;
            ps.ground_entity_num = ENTITYNUM_NONE;
            ps.pm_flags |= pm_flags::MC_MAY_GLIDE;
            let mut cmd = jump;
            assert!(!prepare(&mut ps, &mut cmd, 0, liquid, climbing));
        }
    }

    #[test]
    fn water_climbables_flight_and_a_worn_out_elytra_stop_the_glide() {
        for (liquid, climbing, flags) in [
            (true, false, 0),
            (false, true, 0),
            (false, false, pm_flags::MC_FLYING),
            (false, false, pm_flags::MANTLE),
        ] {
            let mut ps = glider(100.0);
            ps.pm_flags |= flags;
            let mut cmd = UserCmd::default();
            assert!(!prepare(&mut ps, &mut cmd, 0, liquid, climbing));
            assert_eq!(ps.pm_flags & pm_flags::MC_GLIDING, 0);
        }
        let mut worn = glider(100.0);
        worn.pm_flags &= !pm_flags::MC_MAY_GLIDE;
        let mut cmd = UserCmd::default();
        assert!(!prepare(&mut worn, &mut cmd, 0, false, false));
        assert_eq!(worn.pm_flags & pm_flags::MC_GLIDING, 0);
    }

    #[test]
    fn landing_ends_the_glide() {
        let mut ps = glider(3.0 * WATER_BLOCK);
        glide_frames(&mut ps, 50, 40, [0.5, 0.0, -0.866], None, &Floor);
        assert_eq!(ps.pm_flags & pm_flags::MC_GLIDING, 0);
        assert_ne!(ps.ground_entity_num, ENTITYNUM_NONE);
        assert!(ps.origin[2].abs() < 1.0, "{:?}", ps.origin);
    }

    #[test]
    fn a_wall_takes_the_horizontal_speed_but_not_the_glide() {
        let mut ps = glider(100.0 * WATER_BLOCK);
        ps.velocity = [(1.5 * PER_TICK) as f32, 0.0, 0.0];
        let wall = Wall(3.0 * WATER_BLOCK);
        let mut hit = None;
        for frame in 0..20 {
            let before = per_tick(ps.velocity)[0];
            glide_frames(&mut ps, 16, 1, [1.0, 0.0, 0.0], None, &wall);
            let after = per_tick(ps.velocity)[0];
            if before - after > 0.5 {
                hit.get_or_insert((frame, before, after));
            }
        }
        let (_, before, after) = hit.expect("the glider reaches the wall");
        assert!(before > 1.4 && after.abs() < 1e-3, "{before} {after}");
        assert_ne!(ps.pm_flags & pm_flags::MC_GLIDING, 0);
    }

    #[test]
    fn cobwebs_hold_a_glider_without_momentum() {
        let web = Some([0.25, 0.25, 0.05]);
        for msec in [50, 8] {
            let top = 100.0 * WATER_BLOCK;
            let mut ps = glider(top);
            ps.velocity = [(1.5 * PER_TICK) as f32, 0.0, 0.0];
            glide_frames(&mut ps, msec, 1000 / msec, [1.0, 0.0, 0.0], web, &Open);
            assert_eq!(ps.velocity, [0.0; 3]);
            let fallen = (top - ps.origin[2]) / WATER_BLOCK;
            assert!(fallen < 0.1, "msec {msec}: {fallen}");
        }
    }
}
