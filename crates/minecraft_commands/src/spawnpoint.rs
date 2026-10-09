use crate::args::{Arg, ArgKind, WorldCoordinate};
use crate::blocks::block_pos;
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::Ctx;
use crate::text::Text;
use crate::tracker::Tracker;
use crate::tree::Node;

#[derive(Clone, Debug, PartialEq)]
pub struct Respawn {
    pub dimension: String,
    pub pos: [i32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub forced: bool,
}

pub fn wrap_degrees(degrees: f32) -> f32 {
    let mut wrapped = degrees % 360.0;
    if wrapped >= 180.0 {
        wrapped -= 360.0;
    }
    if wrapped < -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

pub fn java_float(value: f32) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    if value == 0.0 {
        return if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        }
        .to_owned();
    }
    if (1e-3..1e7).contains(&value.abs()) {
        let text = value.to_string();
        return if text.contains('.') {
            text
        } else {
            format!("{text}.0")
        };
    }
    let text = format!("{value:e}");
    let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
    if mantissa.contains('.') {
        format!("{mantissa}E{exponent}")
    } else {
        format!("{mantissa}.0E{exponent}")
    }
}

pub fn in_spawnable_bounds(pos: [i32; 3]) -> bool {
    let horizontal = |axis: i32| (-30_000_000..30_000_000).contains(&axis);
    horizontal(pos[0]) && horizontal(pos[2]) && (-20_000_000..20_000_000).contains(&pos[1])
}

pub(crate) fn node() -> Node {
    Node::literal("spawnpoint")
        .requires(GAMEMASTERS)
        .executes(spawnpoint)
        .then(
            Node::argument("targets", ArgKind::Players { single: false })
                .executes(spawnpoint)
                .then(
                    Node::argument("pos", ArgKind::BlockPos)
                        .executes(spawnpoint)
                        .then(Node::argument("rotation", ArgKind::Rotation).executes(spawnpoint)),
                ),
        )
}

fn angle(coordinate: WorldCoordinate, base: f32) -> f32 {
    if coordinate.relative {
        (f64::from(base) + coordinate.value) as f32
    } else {
        coordinate.value as f32
    }
}

fn spawnpoint(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let targets = ctx.targets("targets")?;
    let pos = match ctx.arg("pos") {
        Some(Arg::BlockPos(coordinates)) => {
            let pos = block_pos(coordinates, ctx);
            if !in_spawnable_bounds(pos) {
                return Err(CommandError::new("argument.pos.outofbounds", Vec::new()));
            }
            pos
        }
        _ => ctx.pose().feet.map(|axis| axis.floor() as i32),
    };
    let (yaw, pitch) = match ctx.arg("rotation") {
        Some(Arg::Rotation([yaw, pitch])) => {
            let pose = ctx.pose();
            (angle(*yaw, pose.yaw), angle(*pitch, pose.pitch))
        }
        _ => (0.0, 0.0),
    };
    let respawn = Respawn {
        dimension: ctx.dimension(),
        pos,
        yaw: wrap_degrees(yaw),
        pitch: pitch.clamp(-90.0, 90.0),
        forced: true,
    };
    let mut tracker = Tracker::default();
    for player in targets {
        if ctx.world.set_respawn(player, &respawn) {
            tracker.track(player, 1);
        }
    }
    if tracker.count(false) == 0 {
        return Err(CommandError::new(
            "argument.entity.notfound.player",
            Vec::new(),
        ));
    }
    let [x, y, z] = respawn.pos;
    let mut args = vec![
        literal(x),
        literal(y),
        literal(z),
        literal(java_float(respawn.yaw)),
        literal(java_float(respawn.pitch)),
        literal(&respawn.dimension),
    ];
    let key = match tracker.single(true) {
        Some(only) => {
            args.push(ctx.display_name(only));
            "commands.spawnpoint.success.single"
        }
        None => {
            args.push(literal(tracker.count(true)));
            "commands.spawnpoint.success.multiple"
        }
    };
    ctx.success(Text::translate(key, args), true);
    Ok(tracker.total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::Pose;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};

    fn world() -> FakeWorld {
        FakeWorld {
            respawns: vec![(HOST, None)],
            poses: vec![(
                HOST,
                Pose {
                    feet: [10.7, 64.0, -3.2],
                    yaw: 350.0,
                    pitch: 80.0,
                },
            )],
            ..FakeWorld::default()
        }
    }

    #[test]
    fn floats_print_like_java() {
        assert_eq!(
            [0.0, -0.0, 90.0, -45.5, 0.1, 1e-4, 1.5e7, 12345.678].map(java_float),
            [
                "0.0",
                "-0.0",
                "90.0",
                "-45.5",
                "0.1",
                "1.0E-4",
                "1.5E7",
                "12345.678"
            ]
        );
        assert_eq!(
            [0.0, 180.0, -180.0, 350.0, 720.5, -190.0].map(wrap_degrees),
            [0.0, -180.0, -180.0, -10.0, 0.5, 170.0]
        );
        assert!(in_spawnable_bounds([29_999_999, -19_999_999, -30_000_000]));
        assert!(!in_spawnable_bounds([0, 20_000_000, 0]));
        assert!(!in_spawnable_bounds([30_000_000, 0, 0]));
    }

    #[test]
    fn spawnpoint_passes_seven_arguments_like_vanilla() {
        let mut server = server();
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "spawnpoint");
        assert_eq!(
            texts(&out),
            vec!["Set spawn point to 10, 64, -4 [0.0] in 0.0 for minecraft:overworld"]
        );
        assert_eq!(
            world.respawns[0].1,
            Some(Respawn {
                dimension: "minecraft:overworld".into(),
                pos: [10, 64, -4],
                yaw: 0.0,
                pitch: 0.0,
                forced: true,
            })
        );
        let out = run(&mut server, &mut world, HOST, "spawnpoint @s 1 2 3 90 -45");
        assert_eq!(
            texts(&out),
            vec!["Set spawn point to 1, 2, 3 [90.0] in -45.0 for minecraft:overworld"]
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "spawnpoint Steve ~ ~ ~ ~10 ~20",
        );
        let respawn = world.respawns[0].1.clone().unwrap();
        assert_eq!((respawn.yaw, respawn.pitch), (0.0, 90.0));
        let out = run(&mut server, &mut world, HOST, "spawnpoint @s 0 30000000 0");
        assert_eq!(
            texts(&out),
            vec!["That position is outside the allowed boundaries"]
        );
        let out = run(&mut server, &mut world, HOST, "spawnpoint @s 0 0 0 90");
        assert_eq!(texts(&out)[0], "Incomplete (expected 2 coordinates)");
    }

    #[test]
    fn spawnpoint_counts_players_and_reports_missing_ones() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        world.respawns.push((FRIEND, None));
        let out = run(&mut server, &mut world, HOST, "spawnpoint @a 5 70 5");
        assert_eq!(
            texts(&out),
            vec!["Set spawn point to 5, 70, 5 [0.0] in 0.0 for minecraft:overworld players"]
        );
        assert!(world.respawns.iter().all(|(_, respawn)| respawn.is_some()));
        let out = run(&mut server, &mut FakeWorld::default(), HOST, "spawnpoint");
        assert_eq!(texts(&out), vec!["No player was found"]);
        let out = run(&mut server, &mut world, FRIEND, "spawnpoint");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }
}
