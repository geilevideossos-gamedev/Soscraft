use playerstate_iw4::{PlayerState, UserCmd};

use crate::{CollisionBackend, MoveBounds, WATER_BLOCK, sneaking};

const BACK_OFF_STEP: f32 = 0.05 * WATER_BLOCK;
pub const MAX_UP_STEP: f32 = 0.6 * WATER_BLOCK;
const MAX_BACK_OFF_DISTANCE: f32 = 8.0 * WATER_BLOCK;

pub fn back_off_from_edge<C: CollisionBackend>(
    ps: &mut PlayerState,
    cmd: &UserCmd,
    frametime: f32,
    bounds: MoveBounds,
    collision: &C,
) {
    if !sneaking(ps, cmd) || ps.velocity[2] > 0.0 || frametime <= 0.0 {
        return;
    }
    let wanted = [ps.velocity[0] * frametime, ps.velocity[1] * frametime];
    if !wanted
        .iter()
        .all(|distance| distance.is_finite() && distance.abs() <= MAX_BACK_OFF_DISTANCE)
    {
        return;
    }
    let origin = ps.origin;
    let ground = |dx: f32, dy: f32| {
        let shifted = [origin[0] + dx, origin[1] + dy, origin[2]];
        collision.ground_within(shifted, bounds.mins, bounds.maxs, MAX_UP_STEP)
    };
    if ground(0.0, 0.0) != Some(true) {
        return;
    }
    let can_fall = |dx: f32, dy: f32| ground(dx, dy) == Some(false);
    let mut kept = wanted;
    while kept[0] != 0.0 && can_fall(kept[0], 0.0) {
        kept[0] = shrink(kept[0]);
    }
    while kept[1] != 0.0 && can_fall(0.0, kept[1]) {
        kept[1] = shrink(kept[1]);
    }
    while kept[0] != 0.0 && kept[1] != 0.0 && can_fall(kept[0], kept[1]) {
        kept = kept.map(shrink);
    }
    for k in 0..2 {
        if kept[k] != wanted[k] {
            ps.velocity[k] = kept[k] / frametime;
        }
    }
}

fn shrink(distance: f32) -> f32 {
    if distance.abs() <= BACK_OFF_STEP {
        0.0
    } else if distance > 0.0 {
        distance - BACK_OFF_STEP
    } else {
        distance + BACK_OFF_STEP
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::GroundTraceInput;
    use playerstate_iw4::{buttons, pm_flags};

    const BOUNDS: MoveBounds = MoveBounds {
        mins: [-10.8, -10.8, 0.0],
        maxs: [10.8, 10.8, 64.8],
        tracemask: 0,
        heights: [64.8, 54.0, 30.0],
    };
    const FRAME: f32 = 0.05;

    struct Ledge {
        edge: f32,
        lower_drop: Option<f32>,
    }

    fn open(input: GroundTraceInput) -> trace_iw4::Trace {
        trace_iw4::Trace {
            fraction: 1.0,
            endpos: input.end,
            ..trace_iw4::Trace::default()
        }
    }

    impl CollisionBackend for Ledge {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            open(input)
        }

        fn ground_within(
            &self,
            origin: [f32; 3],
            mins: [f32; 3],
            _maxs: [f32; 3],
            drop: f32,
        ) -> Option<bool> {
            let over_ledge = origin[0] + mins[0] < self.edge;
            let lower = self.lower_drop.is_some_and(|lower| lower <= drop);
            Some(over_ledge || lower)
        }
    }

    struct Brushes;

    impl CollisionBackend for Brushes {
        fn trace(&self, input: GroundTraceInput) -> trace_iw4::Trace {
            open(input)
        }
    }

    fn crouched_at(x: f32, velocity: [f32; 3]) -> PlayerState {
        let mut ps = PlayerState::ZERO;
        ps.origin = [x, 0.0, 0.0];
        ps.velocity = velocity;
        ps.pm_flags |= pm_flags::CROUCH;
        ps
    }

    const CLIFF: Ledge = Ledge {
        edge: 100.0,
        lower_drop: None,
    };

    #[test]
    fn a_sneaking_player_stops_while_the_hull_still_overlaps_the_ledge() {
        let mut ps = crouched_at(108.0, [60.0, 0.0, 0.0]);
        back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        let next = ps.origin[0] + ps.velocity[0] * FRAME;
        assert!(next + BOUNDS.mins[0] < CLIFF.edge, "{next}");
        assert!(ps.velocity[0] > 0.0, "{:?}", ps.velocity);
        let mut at_edge = crouched_at(110.5, [60.0, 0.0, 0.0]);
        back_off_from_edge(&mut at_edge, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        assert_eq!(at_edge.velocity[0], 0.0);
    }

    #[test]
    fn walking_along_or_away_from_the_edge_is_untouched() {
        let mut ps = crouched_at(110.5, [-60.0, 40.0, 0.0]);
        back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        assert_eq!(ps.velocity, [-60.0, 40.0, 0.0]);
        let mut diagonal = crouched_at(110.5, [60.0, 40.0, 0.0]);
        back_off_from_edge(&mut diagonal, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        assert_eq!(diagonal.velocity, [0.0, 40.0, 0.0]);
    }

    #[test]
    fn only_sneaking_on_the_ground_backs_off() {
        let mut standing = crouched_at(110.5, [60.0, 0.0, 0.0]);
        standing.pm_flags &= !pm_flags::CROUCH;
        back_off_from_edge(&mut standing, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        assert_eq!(standing.velocity[0], 60.0);
        standing.velocity[2] = 10.0;
        let holding_crouch = UserCmd {
            buttons: buttons::CROUCH,
            ..UserCmd::default()
        };
        back_off_from_edge(&mut standing, &holding_crouch, FRAME, BOUNDS, &CLIFF);
        assert_eq!(standing.velocity[0], 60.0);
        standing.velocity[2] = 0.0;
        back_off_from_edge(&mut standing, &holding_crouch, FRAME, BOUNDS, &CLIFF);
        assert_eq!(standing.velocity[0], 0.0);
    }

    #[test]
    fn a_drop_within_the_step_height_is_walked_down() {
        let slab = Ledge {
            edge: 100.0,
            lower_drop: Some(0.5 * WATER_BLOCK),
        };
        let mut ps = crouched_at(110.5, [60.0, 0.0, 0.0]);
        back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &slab);
        assert_eq!(ps.velocity[0], 60.0);
        let block = Ledge {
            edge: 100.0,
            lower_drop: Some(WATER_BLOCK),
        };
        back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &block);
        assert_eq!(ps.velocity[0], 0.0);
    }

    #[test]
    fn players_held_up_by_something_other_than_blocks_are_untouched() {
        let mut on_a_head = crouched_at(150.0, [60.0, 40.0, 0.0]);
        back_off_from_edge(&mut on_a_head, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
        assert_eq!(on_a_head.velocity, [60.0, 40.0, 0.0]);
    }

    #[test]
    fn runaway_speeds_are_left_alone() {
        for speed in [f32::NAN, f32::INFINITY, 1.0e9] {
            let mut ps = crouched_at(110.5, [speed, 0.0, 0.0]);
            back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &CLIFF);
            assert!(ps.velocity[0].is_nan() || ps.velocity[0] == speed);
        }
    }

    #[test]
    fn maps_without_block_ground_are_untouched() {
        let mut ps = crouched_at(110.5, [60.0, 0.0, 0.0]);
        back_off_from_edge(&mut ps, &UserCmd::default(), FRAME, BOUNDS, &Brushes);
        assert_eq!(ps.velocity[0], 60.0);
    }
}
