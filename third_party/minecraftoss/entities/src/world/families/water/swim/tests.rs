use super::*;
use crate::look::LookControl;
use crate::movement::Body;
use minecraftoss_player::{Block, Pos, World};

const DOLPHIN: SmoothSwimmingMoveControl = SmoothSwimmingMoveControl::new(85, 10, 0.02, 0.1, true);
const START: DVec3 = DVec3::new(0.5, 64.0, 0.5);
const EYE: f32 = 0.3;

struct Sea;

impl World for Sea {
    fn block(&self, _: Pos) -> Option<Block> {
        Some(Block::new("minecraft:water"))
    }

    fn set_block(&mut self, _: Pos, _: Option<Block>) {}
}

fn moving_to(offset: DVec3) -> MoveControl {
    let mut control = MoveControl::default();
    control.set_wanted_position(START + offset, 1.0);
    control
}

fn smooth(control: &MoveControl, in_water: bool, frame: &mut SwimFrame, velocity: &mut DVec3) {
    DOLPHIN.tick(control, false, START, in_water, 1.2, frame, velocity);
}

fn fish(control: &MoveControl, eyes_in_water: bool, frame: &mut SwimFrame, velocity: &mut DVec3) {
    FishMoveControl.tick(control, false, START, eyes_in_water, 0.7, frame, velocity);
}

fn facing(yaw: f32) -> SwimFrame {
    SwimFrame {
        yaw,
        ..SwimFrame::default()
    }
}

#[test]
fn smooth_swimming_turns_ten_degrees_and_pitches_five_up_to_eighty_five() {
    let speed = 1.2_f32;
    let east_and_up = moving_to(DVec3::new(10.0, 10.0, 0.0));
    let (mut frame, mut velocity) = (facing(0.0), DVec3::ZERO);
    smooth(&east_and_up, true, &mut frame, &mut velocity);
    assert_eq!(frame.yaw, 350.0, "ten degrees towards the east");
    assert_eq!((frame.body_yaw, frame.head_yaw), (350.0, 350.0));
    assert_eq!(frame.pitch, -5.0, "five degrees nose up");
    assert_eq!(frame.speed, speed * 0.02, "its speed in water");
    let radians = f64::from(-5.0_f32 * DEG_TO_RAD);
    assert_eq!(frame.forward, mth::cos(radians) * speed);
    assert_eq!(frame.up, -mth::sin(radians) * speed);
    assert!(frame.up > 0.0 && frame.forward > frame.up);
    assert_eq!(
        velocity,
        DVec3::new(0.0, 0.005, 0.0),
        "it floats up a little"
    );
    for _ in 0..20 {
        smooth(&east_and_up, true, &mut frame, &mut velocity);
    }
    let forty_five = -((mth::atan2(10.0, 10.0) * RAD_TO_DEG) as f32);
    assert_eq!(
        (frame.yaw, frame.pitch),
        (270.0, forty_five),
        "facing its target"
    );

    let straight_up = moving_to(DVec3::new(0.1, 10.0, 0.0));
    let mut frame = facing(270.0);
    for tick in 1..=20 {
        smooth(&straight_up, true, &mut frame, &mut velocity);
        assert_eq!(frame.pitch, -5.0 * tick.min(17) as f32, "tick {tick}");
    }

    let east = moving_to(DVec3::new(10.0, 0.0, 0.0));
    let (mut frame, mut velocity) = (facing(0.0), DVec3::ZERO);
    frame.up = 0.3;
    smooth(&east, false, &mut frame, &mut velocity);
    assert_eq!(frame.yaw, 350.0);
    assert_eq!(
        (frame.speed, frame.forward),
        (0.0, 0.0),
        "too far round to swim on land"
    );
    assert_eq!((frame.pitch, frame.up, velocity), (0.0, 0.3, DVec3::ZERO));
    let mut frame = facing(270.0);
    smooth(&east, false, &mut frame, &mut velocity);
    assert_eq!((frame.speed, frame.forward), (speed * 0.1, speed * 0.1));
    let mut frame = facing(315.0);
    smooth(&east, false, &mut frame, &mut velocity);
    assert_eq!(frame.yaw, 305.0);
    assert_eq!(
        frame.speed,
        speed * 0.1 * 0.5,
        "half speed at 35 degrees off"
    );

    let busy = SwimFrame {
        yaw: 12.0,
        pitch: -20.0,
        speed: 0.5,
        sideways: 0.1,
        up: 0.2,
        forward: 0.3,
        ..SwimFrame::default()
    };
    let mut frame = busy;
    smooth(&MoveControl::default(), true, &mut frame, &mut velocity);
    let stopped = SwimFrame {
        speed: 0.0,
        sideways: 0.0,
        up: 0.0,
        forward: 0.0,
        ..busy
    };
    assert_eq!(frame, stopped, "no move: all its inputs stop");
    assert_eq!(velocity.y, 0.005);
    let mut frame = busy;
    DOLPHIN.tick(&east, true, START, true, 1.2, &mut frame, &mut velocity);
    assert_eq!(frame, stopped, "a finished path stops it too");
    let mut frame = busy;
    let there = moving_to(DVec3::new(0.0001, 0.0, 0.0));
    smooth(&there, true, &mut frame, &mut velocity);
    assert_eq!(
        frame,
        SwimFrame {
            forward: 0.0,
            ..busy
        },
        "there already"
    );

    let weightless = SmoothSwimmingMoveControl::new(85, 10, 0.011, 0.0, false);
    let mut velocity = DVec3::ZERO;
    weightless.tick(
        &east,
        false,
        START,
        true,
        1.2,
        &mut facing(0.0),
        &mut velocity,
    );
    assert_eq!(velocity, DVec3::ZERO, "no lift without its gravity");
}

#[test]
fn the_smooth_look_tilts_twenty_and_ten_and_drags_the_body_after_ten_degrees() {
    let target = DVec3::new(10.5, 64.0 + f64::from(EYE), 0.5);
    let mut look = LookControl::new(0.0);
    let mut body = 0.0;
    look.set_look_at_with_limits(target, 10.0, 40.0);
    look.tick_smooth_swimming(START, EYE, &mut body, true, 10.0);
    assert_eq!((look.head_yaw, look.pitch, body), (-10.0, 10.0, 0.0));
    look.tick_smooth_swimming(START, EYE, &mut body, true, 10.0);
    assert_eq!((look.head_yaw, look.pitch, body), (-20.0, 10.0, -4.0));
    look.tick_smooth_swimming(START, EYE, &mut body, false, 10.0);
    assert_eq!((look.head_yaw, look.pitch, body), (-10.0, 5.0, -4.0));
    look.tick_smooth_swimming(START, EYE, &mut body, false, 10.0);
    assert_eq!((look.head_yaw, look.pitch, body), (-4.0, 0.0, -4.0));

    let mut look = LookControl::new(0.0);
    let mut body = 0.0;
    for _ in 0..30 {
        look.set_look_at_with_limits(target, 10.0, 40.0);
        look.tick_smooth_swimming(START, EYE, &mut body, true, 10.0);
    }
    assert_eq!(look.head_yaw, -70.0, "twenty degrees past the target");
    assert_eq!(look.pitch, 10.0, "ten degrees below it");
    assert_eq!(body, -60.0, "the body is dragged to within ten degrees");

    look.tick_smooth_swimming(START, EYE, &mut body, true, 10.0);
    assert_eq!((look.head_yaw, look.pitch, body), (-70.0, 10.0, -60.0));
    look.pitch = 7.0;
    for _ in 0..3 {
        look.tick_smooth_swimming(START, EYE, &mut body, true, 10.0);
    }
    assert_eq!(look.pitch, 7.0, "it keeps its pitch while it swims a path");
    look.tick_smooth_swimming(START, EYE, &mut body, false, 10.0);
    assert_eq!(look.pitch, 2.0);

    let mut fresh = LookControl::new(30.0);
    let mut body = 0.0;
    fresh.tick_smooth_swimming(START, EYE, &mut body, false, 10.0);
    assert_eq!(
        (fresh.head_yaw, body),
        (30.0, 4.0),
        "a head that never looked"
    );
}

#[test]
fn the_fish_control_lerps_its_speed_by_an_eighth_and_rises_towards_its_target() {
    let wanted = 0.7_f32;
    let up_ahead = moving_to(DVec3::new(10.0, 4.0, 0.0));
    let (mut frame, mut velocity) = (facing(0.0), DVec3::ZERO);
    fish(&up_ahead, true, &mut frame, &mut velocity);
    let first = 0.125 * wanted;
    assert_eq!((frame.speed, frame.forward), (first, first));
    let rise = f64::from(first) * (4.0 / 116.0_f64.sqrt()) * 0.1;
    assert_eq!(velocity.y, 0.005 + rise);
    assert_eq!(
        (frame.yaw, frame.body_yaw),
        (270.0, 270.0),
        "one turn of up to ninety"
    );
    assert_eq!((frame.pitch, frame.head_yaw, frame.up), (0.0, 0.0, 0.0));
    fish(&up_ahead, true, &mut frame, &mut velocity);
    assert_eq!(frame.speed, first + 0.125 * (wanted - first));

    let ahead = moving_to(DVec3::new(10.0, 0.0, 0.0));
    let (mut frame, mut velocity) = (facing(0.0), DVec3::ZERO);
    fish(&ahead, false, &mut frame, &mut velocity);
    assert_eq!(velocity, DVec3::ZERO, "dry eyes and a level target");
    let above = moving_to(DVec3::new(0.0, 3.0, 0.0));
    let mut frame = facing(123.0);
    fish(&above, false, &mut frame, &mut velocity);
    assert_eq!(
        (frame.yaw, frame.body_yaw),
        (123.0, 0.0),
        "no turn straight up"
    );
    assert!(velocity.y > 0.0);

    let mut frame = SwimFrame {
        speed: 0.4,
        forward: 0.4,
        ..facing(40.0)
    };
    let mut velocity = DVec3::ZERO;
    fish(&MoveControl::default(), true, &mut frame, &mut velocity);
    assert_eq!((frame.speed, frame.forward, frame.yaw), (0.0, 0.0, 40.0));
    assert_eq!(velocity.y, 0.005, "it still floats with its eyes under");
}

#[test]
fn swimming_travel_keeps_nine_tenths_without_gravity() {
    let mut body = Body::new(START, 0.5, 0.3);
    body.velocity = DVec3::new(0.1, 0.05, 0.0);
    body.travel_swimming(&Sea, DVec3::Z, 0.0, 0.01);
    let moved = DVec3::new(0.1, 0.05, 0.01);
    assert_eq!(body.position, START + moved);
    assert_eq!(
        body.velocity,
        moved * DVec3::splat(0.9),
        "no gravity, no 0.8"
    );

    let mut body = Body::new(START, 0.5, 0.3);
    body.travel_swimming(&Sea, DVec3::Y, 0.0, 0.02);
    assert_eq!(body.position.y, START.y + 0.02, "its up input lifts it");
    assert_eq!(body.velocity, DVec3::new(0.0, 0.02 * 0.9, 0.0));

    let mut sinking = Body::new(START, 0.5, 0.3);
    sinking.travel_water(&Sea, DVec3::ZERO, 0.0);
    assert!(sinking.velocity.y < 0.0, "plain water travel sinks");
}
