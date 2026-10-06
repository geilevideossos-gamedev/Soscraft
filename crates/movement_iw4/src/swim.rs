use playerstate_iw4::{ENTITYNUM_NONE, PlayerState, UserCmd, buttons, pm_flags};

use crate::{CollisionBackend, GroundTraceInput, MoveBounds, Pml, step_slide_move};

pub const WATER_BLOCK: f32 = 36.0;
const TICK_SECONDS: f32 = 0.05;
const PER_TICK: f32 = WATER_BLOCK / TICK_SECONDS;
const SWIM_SPEED: f32 = 0.02;
const INPUT_SCALE: f32 = 0.98;
const WATER_DRAG: f32 = 0.8;
const SPRINT_DRAG: f32 = 0.9;
const VERTICAL_DRAG: f32 = 0.8;
const SINK_GRAVITY: f32 = 0.005;
const RISE: f32 = 0.04;
const DIVE: f32 = 0.04;
const GROUND_JUMP: f32 = 0.42;
const SHALLOW: f32 = 0.4;
const GROUND_JUMP_DELAY_MS: i32 = 500;
const LEAP: f32 = 0.3;
const LEAP_CLEARANCE: f32 = 0.6;
const LOOK_STEEP: f32 = -0.2;
const LOOK_PULL: f32 = 0.06;
const LOOK_PULL_STEEP: f32 = 0.085;
const LAVA_DRAG: f32 = 0.5;
const SHALLOW_LAVA_VERTICAL_DRAG: f32 = 0.8;
const LAVA_GRAVITY: f32 = 0.02;
const SNEAK_SPEED: f32 = 0.3;
const BUTTON_JUMP: u32 = 0x400;
const BUTTON_SPRINT: u32 = 0x2;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WaterSample {
    pub depth: f32,
    pub eye: bool,
    pub head: bool,
}

impl WaterSample {
    #[must_use]
    pub fn wet(&self) -> bool {
        self.depth > 0.0
    }
}

#[must_use]
pub fn swimming(ps: &PlayerState, water: WaterSample) -> bool {
    ps.pm_flags & pm_flags::SPRINTING != 0 && (water.eye || water.head)
}

pub(crate) fn fluid_sink(ps: &PlayerState, water: WaterSample, lava: f32) -> Option<f32> {
    if water.wet() {
        Some(if ps.pm_flags & pm_flags::SPRINTING != 0 { 0.0 } else { SINK_GRAVITY })
    } else if lava > 0.0 {
        Some(if lava <= SHALLOW * WATER_BLOCK { LAVA_GRAVITY + SINK_GRAVITY } else { LAVA_GRAVITY })
    } else {
        None
    }
}

pub(crate) fn hold_sprint(ps: &mut PlayerState, water: WaterSample, msec: i32) {
    if water.wet() && ps.pm_flags & pm_flags::SPRINTING != 0 {
        ps.last_sprint_start = ps.last_sprint_start.wrapping_add(msec);
    }
}

pub(crate) fn sprint_cmd(
    ps: &mut PlayerState,
    cmd: &UserCmd,
    water: WaterSample,
    sprint: &mut crate::SprintContext,
) -> UserCmd {
    let mut out = *cmd;
    if !water.wet() {
        return out;
    }
    out.buttons &= !(BUTTON_JUMP | buttons::CROUCH | buttons::PRONE);
    ps.sprint_button_up_required = 0;
    if ps.pm_flags & pm_flags::SPRINTING != 0 && !swimming(ps, water) {
        crate::end_sprint(ps, cmd);
        ps.sprint_button_up_required = 0;
    }
    if ps.pm_flags & pm_flags::SPRINTING != 0 {
        sprint.sprint_forever = true;
    } else {
        sprint.min_sprint_time_seconds = -1.0;
        if !water.eye {
            out.buttons &= !BUTTON_SPRINT;
        }
    }
    out
}

pub(crate) fn stance_cmd(ps: &PlayerState, cmd: &UserCmd, water: WaterSample) -> Option<UserCmd> {
    (water.wet() && ps.pm_flags & pm_flags::SPRINTING != 0).then_some(UserCmd {
        buttons: cmd.buttons & !(buttons::CROUCH | buttons::PRONE),
        ..*cmd
    })
}

#[must_use]
pub fn settle(value: f32, target: f32, keep: f32, ticks: f32) -> f32 {
    target + (value - target) * libm::powf(keep, ticks)
}

#[must_use]
pub fn swim_push(forward: [f32; 2], right: [f32; 2], forwardmove: i8, rightmove: i8) -> [f32; 2] {
    input_push(forward, right, forwardmove, rightmove, SWIM_SPEED)
}

pub(crate) fn input_push(
    forward: [f32; 2],
    right: [f32; 2],
    forwardmove: i8,
    rightmove: i8,
    speed: f32,
) -> [f32; 2] {
    let along = f32::from(forwardmove) / 127.0 * INPUT_SCALE;
    let across = f32::from(rightmove) / 127.0 * INPUT_SCALE;
    let length = libm::sqrtf(along * along + across * across);
    if length < 1.0e-4 {
        return [0.0; 2];
    }
    let scale = speed / length.max(1.0);
    let flat = |v: [f32; 2]| {
        let n = libm::sqrtf(v[0] * v[0] + v[1] * v[1]);
        if n > 0.0 {
            [v[0] / n, v[1] / n]
        } else {
            [0.0; 2]
        }
    };
    let (forward, right) = (flat(forward), flat(right));
    [
        (forward[0] * along + right[0] * across) * scale,
        (forward[1] * along + right[1] * across) * scale,
    ]
}

#[must_use]
pub fn vertical_pull(look_up: f32, lift: f32, pull: f32, gravity: f32) -> (f32, f32) {
    let keep = VERTICAL_DRAG * (1.0 - pull);
    let target = (lift + pull * look_up - (1.0 - pull) * gravity) / (1.0 - keep);
    (target, keep)
}

pub fn water_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    water: WaterSample,
    bounds: MoveBounds,
    collision: &C,
) {
    let ticks = pml.frametime / TICK_SECONDS;
    let sprinting = ps.pm_flags & pm_flags::SPRINTING != 0;
    let jumping = cmd.buttons & BUTTON_JUMP != 0;
    let diving = ps.pm_flags & (pm_flags::CROUCH | pm_flags::PRONE) != 0
        || cmd.buttons & (buttons::CROUCH | buttons::PRONE) != 0;
    let shallow = water.depth <= SHALLOW * WATER_BLOCK;
    let on_ground = pml.walking != 0;
    let mut v = [
        ps.velocity[0] / PER_TICK,
        ps.velocity[1] / PER_TICK,
        ps.velocity[2] / PER_TICK,
    ];

    let mut lift = 0.0;
    if jumping && on_ground && shallow {
        if cmd.server_time.wrapping_sub(ps.jump_time) >= GROUND_JUMP_DELAY_MS {
            v[2] = GROUND_JUMP;
            ps.jump_time = cmd.server_time;
            leave_ground(ps, pml);
        }
    } else if jumping {
        lift += RISE;
    }
    if diving {
        lift -= DIVE;
    }

    let push = swim_push(
        [pml.forward[0], pml.forward[1]],
        [pml.right[0], pml.right[1]],
        cmd.forwardmove,
        cmd.rightmove,
    );
    let drag = if sprinting { SPRINT_DRAG } else { WATER_DRAG };
    v[0] = settle(v[0], push[0] / (1.0 - drag), drag, ticks);
    v[1] = settle(v[1], push[1] / (1.0 - drag), drag, ticks);

    let look_up = pml.forward[2];
    let pull = if swimming(ps, water) && (look_up <= 0.0 || jumping || water.head) {
        if look_up < LOOK_STEEP {
            LOOK_PULL_STEEP
        } else {
            LOOK_PULL
        }
    } else {
        0.0
    };
    let gravity = if sprinting { 0.0 } else { SINK_GRAVITY };
    let (target, keep) = vertical_pull(look_up, lift, pull, gravity);
    v[2] = settle(v[2], target, keep, ticks);
    if v[2] > 0.0 {
        leave_ground(ps, pml);
    }

    ps.velocity = [v[0] * PER_TICK, v[1] * PER_TICK, v[2] * PER_TICK];
    if pml.walking != 0 {
        crate::back_off_from_edge(ps, cmd, pml.frametime, bounds, collision);
        v = ps.velocity.map(|speed| speed / PER_TICK);
    }
    let start = ps.origin;
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    leap_out_of_fluid(ps, pml, v, start, bounds, collision);
}

pub fn lava_move<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &mut Pml,
    cmd: &UserCmd,
    depth: f32,
    bounds: MoveBounds,
    collision: &C,
) {
    let ticks = pml.frametime / TICK_SECONDS;
    let jumping = cmd.buttons & BUTTON_JUMP != 0;
    let sneaking = ps.pm_flags & (pm_flags::CROUCH | pm_flags::PRONE) != 0;
    let shallow = depth <= SHALLOW * WATER_BLOCK;
    let mut v = [
        ps.velocity[0] / PER_TICK,
        ps.velocity[1] / PER_TICK,
        ps.velocity[2] / PER_TICK,
    ];

    let mut lift = 0.0;
    let mut jumped = false;
    if jumping && pml.walking != 0 && shallow {
        if cmd.server_time.wrapping_sub(ps.jump_time) >= GROUND_JUMP_DELAY_MS {
            v[2] = GROUND_JUMP;
            ps.jump_time = cmd.server_time;
            leave_ground(ps, pml);
            jumped = true;
        }
    } else if jumping {
        lift = RISE;
    }

    let push = swim_push(
        [pml.forward[0], pml.forward[1]],
        [pml.right[0], pml.right[1]],
        cmd.forwardmove,
        cmd.rightmove,
    );
    let input = if sneaking { SNEAK_SPEED } else { 1.0 };
    v[0] = settle(v[0], push[0] * input / (1.0 - LAVA_DRAG), LAVA_DRAG, ticks);
    v[1] = settle(v[1], push[1] * input / (1.0 - LAVA_DRAG), LAVA_DRAG, ticks);
    if !jumped {
        let (keep, sink) = if shallow {
            (SHALLOW_LAVA_VERTICAL_DRAG, LAVA_GRAVITY + SINK_GRAVITY)
        } else {
            (LAVA_DRAG, LAVA_GRAVITY)
        };
        v[2] = settle(v[2], (lift - sink) / (1.0 - keep), keep, ticks);
    }
    if v[2] > 0.0 {
        leave_ground(ps, pml);
    }

    ps.velocity = [v[0] * PER_TICK, v[1] * PER_TICK, v[2] * PER_TICK];
    if pml.walking != 0 {
        crate::back_off_from_edge(ps, cmd, pml.frametime, bounds, collision);
        v = ps.velocity.map(|speed| speed / PER_TICK);
    }
    let start = ps.origin;
    step_slide_move(
        ps,
        pml,
        collision,
        bounds.mins,
        bounds.maxs,
        bounds.tracemask,
        None,
    );
    leap_out_of_fluid(ps, pml, v, start, bounds, collision);
}

fn leave_ground(ps: &mut PlayerState, pml: &mut Pml) {
    ps.ground_entity_num = ENTITYNUM_NONE;
    pml.walking = 0;
    pml.ground_plane = 0;
    pml.almost_ground_plane = 0;
}

fn leap_out_of_fluid<C: CollisionBackend>(
    ps: &mut PlayerState,
    pml: &Pml,
    v: [f32; 3],
    start: [f32; 3],
    bounds: MoveBounds,
    collision: &C,
) {
    let wanted = [
        v[0] * PER_TICK * pml.frametime,
        v[1] * PER_TICK * pml.frametime,
    ];
    let moved = [ps.origin[0] - start[0], ps.origin[1] - start[1]];
    let wanted_length = libm::sqrtf(wanted[0] * wanted[0] + wanted[1] * wanted[1]);
    let moved_length = libm::sqrtf(moved[0] * moved[0] + moved[1] * moved[1]);
    if wanted_length > 0.01 && moved_length < wanted_length * 0.9 {
        let raised = [
            ps.origin[0] + v[0] * WATER_BLOCK,
            ps.origin[1] + v[1] * WATER_BLOCK,
            start[2] + LEAP_CLEARANCE * WATER_BLOCK,
        ];
        let free = collision
            .trace(GroundTraceInput {
                start: raised,
                end: raised,
                mins: bounds.mins,
                maxs: bounds.maxs,
                tracemask: bounds.tracemask,
            })
            .startsolid
            == 0;
        if free
            && !collision
                .water(raised, bounds.mins, bounds.maxs, ps.view_height_current)
                .wet()
            && collision.lava(raised, bounds.mins, bounds.maxs) <= 0.0
        {
            ps.velocity[2] = LEAP * PER_TICK;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn steady(push: f32, drag: f32, ticks: f32, frames: u32) -> f32 {
        let mut v = 0.0;
        for _ in 0..frames {
            v = settle(v, push / (1.0 - drag), drag, ticks);
        }
        v
    }

    #[test]
    fn swimming_reaches_vanilla_speeds() {
        let push = swim_push([1.0, 0.0], [0.0, -1.0], 127, 0)[0];
        assert!((push - 0.0196).abs() < 1e-6);
        let swim = steady(push, WATER_DRAG, 1.0, 200);
        assert!((swim - 0.098).abs() < 1e-4, "{swim}");
        let sprint = steady(push, SPRINT_DRAG, 1.0, 400);
        assert!((sprint - 0.196).abs() < 1e-4, "{sprint}");
    }

    #[test]
    fn many_short_frames_land_where_one_tick_does() {
        let push = 0.0196;
        let one = settle(0.05, push / 0.2, WATER_DRAG, 1.0);
        let mut five = 0.05;
        for _ in 0..5 {
            five = settle(five, push / 0.2, WATER_DRAG, 0.2);
        }
        assert!((one - five).abs() < 1e-6);
        let vanilla = WATER_DRAG * 0.05 + push;
        assert!((one - vanilla).abs() < 1e-6);
    }

    #[test]
    fn diagonal_input_is_not_faster() {
        let straight = swim_push([1.0, 0.0], [0.0, -1.0], 127, 0);
        let diagonal = swim_push([1.0, 0.0], [0.0, -1.0], 127, 127);
        let length = |v: [f32; 2]| libm::sqrtf(v[0] * v[0] + v[1] * v[1]);
        assert!(length(diagonal) <= SWIM_SPEED + 1e-6);
        assert!(length(diagonal) > length(straight));
        assert_eq!(swim_push([1.0, 0.0], [0.0, -1.0], 0, 0), [0.0; 2]);
    }

    #[test]
    fn idle_players_sink_slowly_and_jumping_rises() {
        let (sink, keep) = vertical_pull(0.0, 0.0, 0.0, SINK_GRAVITY);
        assert!((sink + 0.025).abs() < 1e-6);
        assert!((keep - VERTICAL_DRAG).abs() < 1e-6);
        let (rise, _) = vertical_pull(0.0, RISE, 0.0, SINK_GRAVITY);
        assert!((rise - 0.175).abs() < 1e-5, "{rise}");
        let (dive, _) = vertical_pull(0.0, -DIVE, 0.0, SINK_GRAVITY);
        assert!(dive < sink);
    }

    #[test]
    fn a_vanilla_tick_of_vertical_motion_is_matched() {
        let (lift, pull, look, gravity) = (RISE, LOOK_PULL_STEEP, -0.7, SINK_GRAVITY);
        let moving = 0.1_f32;
        let after_drag = VERTICAL_DRAG * moving - gravity;
        let vanilla = after_drag + lift + (look - after_drag) * pull;
        let (target, keep) = vertical_pull(look, lift, pull, gravity);
        assert!((settle(moving, target, keep, 1.0) - vanilla).abs() < 1e-6);
    }

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

    fn tick_pml(walking: bool) -> Pml {
        Pml {
            forward: [1.0, 0.0, 0.0],
            right: [0.0, -1.0, 0.0],
            up: [0.0, 0.0, 1.0],
            frametime: TICK_SECONDS,
            msec: 50,
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

    fn afloat() -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.ground_entity_num = ENTITYNUM_NONE;
        ps
    }

    fn lava_ticks(ps: &mut PlayerState, cmd: UserCmd, depth: f32, ticks: u32) -> [f32; 3] {
        for _ in 0..ticks {
            lava_move(ps, &mut tick_pml(false), &cmd, depth, BOUNDS, &Open);
        }
        ps.velocity.map(|v| v / PER_TICK)
    }

    #[test]
    fn lava_reaches_vanilla_speeds() {
        let deep = 2.0 * WATER_BLOCK;
        let forward = UserCmd {
            forwardmove: 127,
            ..UserCmd::default()
        };
        let mut ps = afloat();
        let wading = lava_ticks(&mut ps, forward, deep, 100);
        assert!((wading[0] - 0.0392).abs() < 1e-5, "{wading:?}");
        assert!((wading[2] + 0.04).abs() < 1e-5, "{wading:?}");
        let rising = UserCmd {
            buttons: BUTTON_JUMP,
            ..forward
        };
        let up = lava_ticks(&mut ps, rising, deep, 100);
        assert!((up[2] - 0.04).abs() < 1e-5, "{up:?}");
        let mut sneaking = afloat();
        sneaking.pm_flags |= pm_flags::CROUCH;
        let slow = lava_ticks(&mut sneaking, forward, deep, 100);
        assert!((slow[0] - 0.0392 * SNEAK_SPEED).abs() < 1e-5, "{slow:?}");
        let mut sprinting = afloat();
        sprinting.pm_flags |= pm_flags::SPRINTING;
        let same = lava_ticks(&mut sprinting, forward, deep, 100);
        assert!((same[0] - 0.0392).abs() < 1e-5, "{same:?}");
        let mut shallow = afloat();
        let sinking = lava_ticks(&mut shallow, UserCmd::default(), 0.3 * WATER_BLOCK, 100);
        assert!((sinking[2] + 0.125).abs() < 1e-5, "{sinking:?}");
    }

    #[test]
    fn grounded_players_jump_out_of_shallow_lava_every_half_second() {
        let shallow = 0.3 * WATER_BLOCK;
        let jump = UserCmd {
            server_time: 1000,
            buttons: BUTTON_JUMP,
            ..UserCmd::default()
        };
        let mut ps = PlayerState::ZERO;
        let mut grounded = tick_pml(true);
        lava_move(&mut ps, &mut grounded, &jump, shallow, BOUNDS, &Open);
        assert!(
            (ps.velocity[2] - GROUND_JUMP * PER_TICK).abs() < 1e-3,
            "{:?}",
            ps.velocity
        );
        assert_eq!(
            (ps.jump_time, ps.ground_entity_num, grounded.walking),
            (1000, ENTITYNUM_NONE, 0)
        );
        let soon = UserCmd {
            server_time: 1450,
            ..jump
        };
        let mut again = PlayerState::ZERO;
        again.jump_time = 1000;
        lava_move(
            &mut again,
            &mut tick_pml(true),
            &soon,
            shallow,
            BOUNDS,
            &Open,
        );
        assert!(again.velocity[2] < 0.0, "{:?}", again.velocity);
        assert_eq!(again.jump_time, 1000);
        let mut deep = PlayerState::ZERO;
        deep.jump_time = 1000;
        lava_move(
            &mut deep,
            &mut tick_pml(true),
            &soon,
            2.0 * WATER_BLOCK,
            BOUNDS,
            &Open,
        );
        assert!(
            (deep.velocity[2] / PER_TICK - 0.02).abs() < 1e-5,
            "{:?}",
            deep.velocity
        );
    }

    #[test]
    fn a_grounded_swimmer_rising_in_deep_fluid_leaves_the_ground_and_a_sinking_one_keeps_it() {
        let deep = WaterSample {
            depth: 2.0 * WATER_BLOCK,
            eye: true,
            head: true,
        };
        let rise = UserCmd {
            server_time: 1000,
            buttons: BUTTON_JUMP,
            ..UserCmd::default()
        };
        let on_floor = || {
            let mut ps = PlayerState::ZERO;
            ps.ground_entity_num = i32::from(trace_iw4::ENTITYNUM_WORLD);
            ps
        };
        for lava in [false, true] {
            for (cmd, rising) in [(rise, true), (UserCmd::default(), false)] {
                let mut ps = on_floor();
                let mut pml = tick_pml(true);
                if lava {
                    lava_move(&mut ps, &mut pml, &cmd, deep.depth, BOUNDS, &Open);
                } else {
                    water_move(&mut ps, &mut pml, &cmd, deep, BOUNDS, &Open);
                }
                assert_eq!(ps.velocity[2] > 0.0, rising, "lava {lava}: {:?}", ps.velocity);
                let grounded = (ps.ground_entity_num, pml.walking, pml.ground_plane);
                if rising {
                    assert_eq!(grounded, (ENTITYNUM_NONE, 0, 0), "lava {lava}");
                } else {
                    assert_eq!(grounded, (i32::from(trace_iw4::ENTITYNUM_WORLD), 1, 1), "lava {lava}");
                }
            }
        }
    }

    #[test]
    fn sprint_swimming_needs_the_head_or_eyes_under() {
        let mut ps = PlayerState::ZERO;
        ps.pm_flags |= pm_flags::SPRINTING;
        let under = WaterSample {
            depth: 60.0,
            eye: true,
            head: true,
        };
        let wading = WaterSample {
            depth: 20.0,
            eye: false,
            head: false,
        };
        assert!(swimming(&ps, under));
        assert!(!swimming(&ps, wading));
        ps.pm_flags &= !pm_flags::SPRINTING;
        assert!(!swimming(&ps, under));
    }

    #[test]
    fn shift_starts_a_swim_once_the_eyes_go_under() {
        let context = crate::SprintContext {
            weapon_max_sprint_time: 4000,
            sprint_forever: false,
            min_sprint_time_seconds: 1.0,
            sprint_delay_seconds: 0.0,
            sprint_forward_minimum: 105,
            stand_up_clear: true,
            sprint_recharge_pause_seconds: 0.0,
        };
        let step = |ps: &mut PlayerState, cmd: UserCmd, water: WaterSample| {
            let mut sprint = context;
            let swim = sprint_cmd(ps, &cmd, water, &mut sprint);
            crate::update_sprint(ps, &swim, BUTTON_SPRINT, sprint)
        };
        let surface = WaterSample {
            depth: 60.0,
            eye: false,
            head: true,
        };
        let under = WaterSample {
            depth: 70.0,
            eye: true,
            head: true,
        };
        let wading = WaterSample {
            depth: 20.0,
            eye: false,
            head: false,
        };
        let mut ps = PlayerState::ZERO;
        ps.sprint_button_up_required = 1;
        ps.sprint_delay = 1;
        ps.last_sprint_start = 1000;
        ps.last_sprint_end = 4900;
        ps.sprint_start_max_length = 4000;
        let held = UserCmd {
            server_time: 5000,
            buttons: BUTTON_SPRINT | BUTTON_JUMP,
            forwardmove: 127,
            ..UserCmd::default()
        };
        assert_eq!(step(&mut ps, held, surface), crate::SprintResult::Unchanged);
        assert_eq!(ps.pm_flags & pm_flags::SPRINTING, 0);
        assert_eq!(step(&mut ps, held, under), crate::SprintResult::Started);
        assert!(swimming(&ps, under));
        let dive = UserCmd {
            server_time: 9000,
            buttons: BUTTON_SPRINT | buttons::CROUCH,
            ..held
        };
        assert_eq!(step(&mut ps, dive, surface), crate::SprintResult::Unchanged);
        assert!(swimming(&ps, surface));
        let stance = stance_cmd(&ps, &dive, surface).expect("a swim stance");
        assert_eq!(stance.buttons & buttons::CROUCH, 0);
        assert!(stance_cmd(&ps, &dive, WaterSample::default()).is_none());
        assert_eq!(step(&mut ps, dive, wading), crate::SprintResult::Unchanged);
        assert_eq!(ps.pm_flags & pm_flags::SPRINTING, 0);
        assert_eq!(ps.sprint_button_up_required, 0);
        let mut dry = PlayerState::ZERO;
        dry.sprint_button_up_required = 1;
        let mut sprint = context;
        let same = sprint_cmd(&mut dry, &held, WaterSample::default(), &mut sprint);
        assert_eq!(
            (same, sprint, dry.sprint_button_up_required),
            (held, context, 1)
        );
    }

    #[test]
    fn water_pauses_the_sprint_budget_without_refilling_it() {
        let context = crate::SprintContext {
            weapon_max_sprint_time: 4000,
            sprint_forever: false,
            min_sprint_time_seconds: 1.0,
            sprint_delay_seconds: 0.0,
            sprint_forward_minimum: 105,
            stand_up_clear: true,
            sprint_recharge_pause_seconds: 0.0,
        };
        let wet = WaterSample {
            depth: 20.0,
            eye: false,
            head: false,
        };
        let mut ps = PlayerState::ZERO;
        ps.pm_flags |= pm_flags::SPRINTING;
        ps.last_sprint_start = 1000;
        ps.sprint_start_max_length = 4000;
        let mut cmd = UserCmd {
            server_time: 4900,
            buttons: 0x2,
            forwardmove: 127,
            ..UserCmd::default()
        };
        for _ in 0..100 {
            cmd.server_time += 20;
            hold_sprint(&mut ps, wet, 20);
            crate::update_sprint(&mut ps, &cmd, 0x2, context);
        }
        assert!(ps.pm_flags & pm_flags::SPRINTING != 0);
        assert_eq!(ps.sprint_start_max_length, 4000);
        let mut dry = 0;
        while ps.pm_flags & pm_flags::SPRINTING != 0 {
            cmd.server_time += 20;
            hold_sprint(&mut ps, WaterSample::default(), 20);
            crate::update_sprint(&mut ps, &cmd, 0x2, context);
            dry += 20;
        }
        assert_eq!(dry, 100);
        let mut idle = ps;
        idle.pm_flags &= !pm_flags::SPRINTING;
        let start = idle.last_sprint_start;
        hold_sprint(&mut idle, wet, 20);
        assert_eq!(idle.last_sprint_start, start);
    }
}
