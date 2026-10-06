use playerstate_iw4::{
    ENTITYNUM_NONE, PM_TYPE_LAST_STAND, PlayerState, UserCmd, buttons, pm_flags,
};

use crate::{
    CollisionBackend, MoveBounds, Pml, WalkMoveContext, accelerate, cmd_scale_walk, friction,
    settle, step_slide_move, swim_push,
};

const BLOCK: f32 = crate::WATER_BLOCK;
const TICK_SECONDS: f32 = 0.05;
const PER_TICK: f32 = BLOCK / TICK_SECONDS;
const CLIMB_LIMIT: f32 = 0.15;
const CLIMB_BOOST: f32 = 0.2;
const GRAVITY: f32 = 0.08;
const VERTICAL_DRAG: f32 = 0.98;
const AIR_FRICTION: f32 = 0.91;
const BLOCKED_SHARE: f32 = 0.9;
const MIN_MOVE: f32 = 0.01;
const PM_TYPE_NORMAL: i32 = 0;

pub const CLIMB_SPEED: f32 = CLIMB_LIMIT * PER_TICK;
pub const CLIMB_RISE: f32 = (CLIMB_BOOST - GRAVITY) * VERTICAL_DRAG * PER_TICK;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ClimbSample {
    pub on: bool,
    pub scaffolding: bool,
}

#[must_use]
pub fn sneaking(ps: &PlayerState, cmd: &UserCmd) -> bool {
    ps.pm_flags & (pm_flags::CROUCH | pm_flags::PRONE) != 0
        || cmd.buttons & (buttons::CROUCH | buttons::PRONE) != 0
}

#[must_use]
pub fn climbing(ps: &PlayerState, climb: ClimbSample) -> bool {
    climb.on
        && ps.pm_flags & pm_flags::LADDER == 0
        && matches!(ps.pm_type, PM_TYPE_NORMAL | PM_TYPE_LAST_STAND)
}

#[must_use]
pub fn mantle_buttons(ps: &PlayerState, climb: ClimbSample, held: u32) -> u32 {
    if climbing(ps, climb) {
        held & !buttons::JUMP
    } else {
        held
    }
}

pub fn hold(velocity: &mut [f32; 3], scaffolding: bool, descending: bool) {
    velocity[0] = velocity[0].clamp(-CLIMB_SPEED, CLIMB_SPEED);
    velocity[1] = velocity[1].clamp(-CLIMB_SPEED, CLIMB_SPEED);
    velocity[2] = velocity[2].max(-CLIMB_SPEED);
    if velocity[2] < 0.0 && descending && !scaffolding {
        velocity[2] = 0.0;
    }
}

#[must_use]
pub fn after_move(vertical: f32, rising: bool, ticks: f32) -> f32 {
    if rising {
        CLIMB_RISE
    } else {
        let fall = -GRAVITY * VERTICAL_DRAG / (1.0 - VERTICAL_DRAG);
        settle(vertical / PER_TICK, fall, VERTICAL_DRAG, ticks) * PER_TICK
    }
}

pub fn climb_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    walk: WalkMoveContext,
    climb: ClimbSample,
    bounds: MoveBounds,
    collision: &C,
) {
    let ticks = pml.frametime / TICK_SECONDS;
    if pml.walking != 0 {
        friction(ps, pml);
        walk_push(ps, pml, cmd, walk);
        crate::walk_move_drop_damage_timer(ps, pml.frametime);
        let normal = [
            f32::from_bits(pml.ground_trace[1]),
            f32::from_bits(pml.ground_trace[2]),
            f32::from_bits(pml.ground_trace[3]),
        ];
        crate::project_velocity(&mut ps.velocity, &normal);
    } else {
        let push = swim_push(
            [pml.forward[0], pml.forward[1]],
            [pml.right[0], pml.right[1]],
            cmd.forwardmove,
            cmd.rightmove,
        );
        for (k, push) in push.into_iter().enumerate() {
            let target = push / (1.0 - AIR_FRICTION);
            ps.velocity[k] =
                settle(ps.velocity[k] / PER_TICK, target, AIR_FRICTION, ticks) * PER_TICK;
        }
    }
    let descending = sneaking(ps, cmd);
    hold(&mut ps.velocity, climb.scaffolding, descending);
    if pml.walking != 0 {
        crate::back_off_from_edge(ps, cmd, pml.frametime, bounds, collision);
    }

    let start = ps.origin;
    let wanted = [
        ps.velocity[0] * pml.frametime,
        ps.velocity[1] * pml.frametime,
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
    let rising = rise_after_move(ps, pml, cmd, start, wanted, collision);
    ps.velocity[2] = after_move(ps.velocity[2], rising, ticks);
}

pub(crate) fn rise_after_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    start: [f32; 3],
    wanted: [f32; 2],
    collision: &C,
) -> bool {
    let blocked = (0..2).any(|k| {
        wanted[k].abs() > MIN_MOVE
            && (ps.origin[k] - start[k]).abs() < wanted[k].abs() * BLOCKED_SHARE
    });
    let jumping = cmd.buttons & buttons::JUMP != 0;
    let rising = (jumping || blocked) && collision.climb(ps.origin).on;
    if rising {
        ps.ground_entity_num = ENTITYNUM_NONE;
        pml.walking = 0;
        pml.ground_plane = 0;
        pml.almost_ground_plane = 0;
    }
    rising
}

fn walk_push(ps: &mut PlayerState, pml: &Pml, cmd: &UserCmd, walk: WalkMoveContext) {
    let scale = cmd_scale_walk(ps, cmd, walk.cmd_scale) * crate::damage_scale_walk(ps.damage_timer);
    let flat = |v: [f32; 3]| {
        let n = libm::sqrtf(v[0] * v[0] + v[1] * v[1]);
        if n > 0.0 {
            [v[0] / n, v[1] / n]
        } else {
            [0.0; 2]
        }
    };
    let (forward, right) = (flat(pml.forward), flat(pml.right));
    let (along, across) = (f32::from(cmd.forwardmove), f32::from(cmd.rightmove));
    let wish = [
        across * right[0] + along * forward[0],
        across * right[1] + along * forward[1],
    ];
    let speed = libm::sqrtf(wish[0] * wish[0] + wish[1] * wish[1]);
    if speed <= 0.0 {
        return;
    }
    let wishdir = [wish[0] / speed, wish[1] / speed, 0.0];
    let accel = crate::walk::walk_accel_scale(ps, pml);
    accelerate(
        ps,
        pml,
        &wishdir,
        speed * walk.weapon_move_scale * scale,
        accel,
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GroundTraceInput;

    struct Column {
        floor: f32,
        wall: f32,
        climbable_below: f32,
    }

    impl CollisionBackend for Column {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            let (front, ahead) = (input.start[0] + input.maxs[0], input.end[0] + input.maxs[0]);
            let hit = if input.end[2] < self.floor && input.start[2] >= self.floor {
                Some((
                    (input.start[2] - self.floor) / (input.start[2] - input.end[2]),
                    [0.0, 0.0, 1.0],
                ))
            } else if ahead > self.wall && front <= self.wall {
                Some(((self.wall - front) / (ahead - front), [-1.0, 0.0, 0.0]))
            } else {
                None
            };
            let Some((fraction, normal)) = hit else {
                return trace_iw4::Trace {
                    fraction: 1.0,
                    endpos: input.end,
                    ..trace_iw4::Trace::default()
                };
            };
            let fraction = (fraction - 0.01).max(0.0);
            trace_iw4::Trace {
                fraction,
                endpos: core::array::from_fn(|k| {
                    input.start[k] + (input.end[k] - input.start[k]) * fraction
                }),
                normal,
                walkable: u8::from(normal[2] > 0.7),
                ..trace_iw4::Trace::default()
            }
        }

        fn climb(&self, origin: [f32; 3]) -> ClimbSample {
            ClimbSample {
                on: origin[2] < self.climbable_below,
                scaffolding: false,
            }
        }
    }

    const BOUNDS: MoveBounds = MoveBounds {
        mins: [-10.8, -10.8, 0.0],
        maxs: [10.8, 10.8, 64.8],
        tracemask: 0,
        heights: [64.8, 54.0, 30.0],
    };

    fn walk_context() -> WalkMoveContext {
        let air = crate::AirMoveContext {
            player_spectate_speed_scale: 1.0,
            shellshock_gravity_scale: 1.0,
            shellshock_gravity_bias: 0.0,
        };
        WalkMoveContext {
            cmd_scale: crate::CmdScaleWalkContext {
                player_back_speed_scale: 0.7,
                player_strafe_speed_scale: 0.8,
                player_sprint_speed_scale: 1.5,
                player_last_stand_crawl_speed_scale: 0.2,
                weapon_move_speed_scale: 1.0,
                weapon_ads_move_speed_scale: 1.0,
                shellshock_affects_movement: false,
            },
            weapon_move_scale: 1.0,
            old_buttons: 0,
            jump: crate::JumpLaunchContext {
                jump_height: 39.0,
                dive: false,
                crouch_jump_scale: 1.0,
                jump_ladder_push_vel: 128.0,
            },
            air,
        }
    }

    fn tick_pml(frametime: f32) -> Pml {
        Pml {
            forward: [1.0, 0.0, 0.0],
            right: [0.0, -1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            frametime,
            msec: (frametime * 1000.0) as i32,
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

    fn airborne(z: f32) -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps.origin = [0.0, 0.0, z];
        ps.speed = 190;
        ps
    }

    fn run(ps: &mut PlayerState, cmd: UserCmd, column: &Column, frametime: f32, seconds: f32) {
        let frames = (seconds / frametime) as u32;
        for _ in 0..frames {
            let climb = column.climb(ps.origin);
            if !climbing(ps, climb) {
                break;
            }
            let mut pml = tick_pml(frametime);
            climb_move(ps, &mut pml, &cmd, walk_context(), climb, BOUNDS, column);
        }
    }

    #[test]
    fn the_vanilla_climb_constants_hold() {
        assert!((CLIMB_SPEED - 108.0).abs() < 1e-3, "{CLIMB_SPEED}");
        assert!(
            (CLIMB_RISE / PER_TICK - 0.1176).abs() < 1e-6,
            "{CLIMB_RISE}"
        );
        assert!((CLIMB_RISE - 84.672).abs() < 1e-3);
        let one = after_move(0.0, false, 1.0) / PER_TICK;
        assert!((one + 0.0784).abs() < 1e-6, "{one}");
        let mut v = [500.0, -500.0, -500.0];
        hold(&mut v, false, false);
        assert_eq!(v, [CLIMB_SPEED, -CLIMB_SPEED, -CLIMB_SPEED]);
        let mut rising = [0.0, 0.0, 300.0];
        hold(&mut rising, false, true);
        assert_eq!(rising[2], 300.0);
    }

    #[test]
    fn sneaking_holds_on_a_ladder_but_not_in_scaffolding() {
        let mut ladder = [0.0, 0.0, -50.0];
        hold(&mut ladder, false, true);
        assert_eq!(ladder[2], 0.0);
        let mut scaffolding = [0.0, 0.0, -50.0];
        hold(&mut scaffolding, true, true);
        assert_eq!(scaffolding[2], -50.0);
        let mut ps = PlayerState::ZERO;
        assert!(!sneaking(&ps, &UserCmd::default()));
        ps.pm_flags |= pm_flags::CROUCH;
        assert!(sneaking(&ps, &UserCmd::default()));
        let crouch_key = UserCmd {
            buttons: buttons::CROUCH,
            ..UserCmd::default()
        };
        assert!(sneaking(&PlayerState::ZERO, &crouch_key));
    }

    #[test]
    fn holding_jump_climbs_at_the_vanilla_speed_at_any_frame_rate() {
        let column = Column {
            floor: -1000.0,
            wall: f32::MAX,
            climbable_below: 1000.0,
        };
        let jump = UserCmd {
            buttons: buttons::JUMP,
            ..UserCmd::default()
        };
        for frametime in [0.05, 0.016, 0.007] {
            let mut ps = airborne(0.0);
            run(&mut ps, jump, &column, frametime, 0.1);
            let start = ps.origin[2];
            run(&mut ps, jump, &column, frametime, 1.0);
            let climbed = ps.origin[2] - start;
            assert!(
                (climbed - CLIMB_RISE).abs() < CLIMB_RISE * 0.05,
                "{frametime}: {climbed}"
            );
        }
    }

    #[test]
    fn letting_go_slides_down_at_the_vanilla_limit_and_sneaking_holds() {
        let column = Column {
            floor: -1000.0,
            wall: f32::MAX,
            climbable_below: 1000.0,
        };
        let mut ps = airborne(0.0);
        run(&mut ps, UserCmd::default(), &column, 0.05, 0.5);
        let start = ps.origin[2];
        run(&mut ps, UserCmd::default(), &column, 0.05, 1.0);
        assert!(
            (start - ps.origin[2] - CLIMB_SPEED).abs() < 1e-2,
            "{}",
            start - ps.origin[2]
        );
        let mut held = airborne(0.0);
        held.pm_flags |= pm_flags::CROUCH;
        run(&mut held, UserCmd::default(), &column, 0.05, 1.0);
        assert!(held.origin[2].abs() < 1e-3, "{:?}", held.origin);
    }

    #[test]
    fn climbing_past_the_top_falls_back_to_gravity_and_input_is_capped() {
        let column = Column {
            floor: -1000.0,
            wall: f32::MAX,
            climbable_below: 10.0,
        };
        let cmd = UserCmd {
            buttons: buttons::JUMP,
            forwardmove: 127,
            ..UserCmd::default()
        };
        let mut ps = airborne(9.9);
        ps.velocity[2] = CLIMB_RISE;
        let mut pml = tick_pml(0.05);
        let climb = column.climb(ps.origin);
        climb_move(
            &mut ps,
            &mut pml,
            &cmd,
            walk_context(),
            climb,
            BOUNDS,
            &column,
        );
        let coasting = (CLIMB_RISE / PER_TICK * VERTICAL_DRAG - GRAVITY * VERTICAL_DRAG) * PER_TICK;
        assert!(
            (ps.velocity[2] - coasting).abs() < 1e-3,
            "{:?}",
            ps.velocity
        );
        let mut fast = airborne(0.0);
        fast.velocity = [400.0, 0.0, 0.0];
        let mut pml = tick_pml(0.05);
        let climb = column.climb(fast.origin);
        climb_move(
            &mut fast,
            &mut pml,
            &cmd,
            walk_context(),
            climb,
            BOUNDS,
            &column,
        );
        assert!(fast.velocity[0] <= CLIMB_SPEED, "{:?}", fast.velocity);
        assert!(
            (fast.origin[0] - CLIMB_SPEED * 0.05).abs() < 1e-3,
            "{:?}",
            fast.origin
        );
    }

    #[test]
    fn pushing_into_a_wall_climbs_without_jump() {
        let column = Column {
            floor: -1000.0,
            wall: 20.0,
            climbable_below: 1000.0,
        };
        let forward = UserCmd {
            forwardmove: 127,
            ..UserCmd::default()
        };
        let mut ps = airborne(0.0);
        ps.origin[0] = 9.0;
        run(&mut ps, forward, &column, 0.05, 0.5);
        let start = ps.origin[2];
        run(&mut ps, forward, &column, 0.05, 1.0);
        assert!(
            (ps.origin[2] - start - CLIMB_RISE).abs() < 1e-2,
            "{:?}",
            ps.origin
        );
        let mut free = airborne(0.0);
        let open = Column {
            wall: f32::MAX,
            ..column
        };
        run(&mut free, forward, &open, 0.05, 1.0);
        assert!(free.origin[2] < 0.0, "{:?}", free.origin);
    }

    #[test]
    fn jump_is_kept_from_the_mantle_while_climbing() {
        let ps = PlayerState::ZERO;
        let on = ClimbSample {
            on: true,
            scaffolding: true,
        };
        assert_eq!(mantle_buttons(&ps, on, buttons::JUMP | 1), 1);
        assert_eq!(
            mantle_buttons(&ps, ClimbSample::default(), buttons::JUMP),
            buttons::JUMP
        );
        let mut noclip = PlayerState::ZERO;
        noclip.pm_type = 2;
        assert!(!climbing(&noclip, on));
    }
}
