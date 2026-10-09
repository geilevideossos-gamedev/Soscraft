use crate::args::{Arg, ArgKind, Registry};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::Ctx;
use crate::tree::{Node, Redirect};

const EXECUTE: Redirect = Redirect::Command("execute");

pub(crate) fn node() -> Node {
    Node::literal("execute")
        .requires(GAMEMASTERS)
        .then(Node::literal("run").redirect(Redirect::Root))
        .then(
            Node::literal("in").then(
                Node::argument("dimension", ArgKind::Resource(Registry::Dimension))
                    .redirect_with(EXECUTE, in_dimension),
            ),
        )
        .then(
            Node::literal("positioned")
                .then(Node::argument("pos", ArgKind::Vec3).redirect_with(EXECUTE, positioned)),
        )
}

fn in_dimension(ctx: &mut Ctx) -> Result<(), CommandError> {
    let Some(Arg::Resource { id, .. }) = ctx.arg("dimension").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let Some(target) = ctx.world.dimension_scale(&id) else {
        return Err(CommandError::new(
            "argument.dimension.invalid",
            vec![literal(&id)],
        ));
    };
    let current = ctx.dimension();
    if current == id {
        return Ok(());
    }
    let scale = ctx.world.dimension_scale(&current).unwrap_or(1.0) / target;
    let mut pose = ctx.pose();
    pose.feet[0] *= scale;
    pose.feet[2] *= scale;
    ctx.at = Some(pose);
    ctx.dimension = Some(id);
    Ok(())
}

fn positioned(ctx: &mut Ctx) -> Result<(), CommandError> {
    let Some(Arg::Vec3(pos)) = ctx.arg("pos").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let mut pose = ctx.pose();
    pose.feet = pos.position(pose);
    ctx.at = Some(pose);
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::protocol::PlayerId;
    use crate::server::Pose;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;

    const NETHER: &str = "minecraft:the_nether";

    fn world() -> FakeWorld {
        FakeWorld {
            poses: vec![(
                HOST,
                Pose {
                    feet: [80.0, 64.0, -16.0],
                    yaw: 0.0,
                    pitch: 0.0,
                },
            )],
            ..FakeWorld::default()
        }
    }

    fn in_nether(world: &mut FakeWorld, player: PlayerId) {
        world.player_dimensions.push((player, NETHER.to_owned()));
    }

    #[test]
    fn execute_in_an_unloaded_dimension_answers_like_vanilla() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:the_nether run tp ~ ~ ~",
        );
        assert_eq!(
            texts(&out),
            vec!["Unknown dimension 'minecraft:the_nether'"]
        );
        assert_eq!(out[0].2, RED);
        assert!(world.teleported.is_empty() && world.dimension_teleports.is_empty());
        let out = run(&mut server, &mut world, HOST, "execute in the_end run seed");
        assert_eq!(texts(&out), vec!["Unknown dimension 'minecraft:the_end'"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:overworld run tp ~ ~10 ~",
        );
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 80.000000, 74.000000, -16.000000"]
        );
        let out = run(&mut server, &mut world, HOST, "execute run seed");
        assert_eq!(texts(&out), vec!["Seed: [-4172144997902289642]"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:overworld run execute run seed",
        );
        assert_eq!(out.len(), 1);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:overworld",
        );
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        let out = run(&mut server, &mut world, FRIEND, "execute run seed");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }

    #[test]
    fn execute_in_a_loaded_dimension_scales_x_and_z() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        world.levels.push((NETHER.to_owned(), 8.0));
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:the_nether run tp ~ ~ ~",
        );
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 10.000000, 64.000000, -2.000000"]
        );
        assert_eq!(
            world.dimension_teleports,
            vec![(HOST, NETHER.to_owned(), [10.0, 64.0, -2.0])]
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:the_nether run locate biome minecraft:desert",
        );
        let request = world.located_requests.last().unwrap();
        assert_eq!(
            (request.dimension.as_str(), request.origin),
            (NETHER, [10, 64, -2])
        );
        in_nether(&mut world, HOST);
        world.poses.push((
            HOST,
            Pose {
                feet: [10.0, 64.0, -2.0],
                yaw: 0.0,
                pitch: 0.0,
            },
        ));
        run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:overworld run tp ~ ~ ~",
        );
        assert_eq!(
            world.dimension_teleports[1],
            (HOST, "minecraft:overworld".to_owned(), [80.0, 64.0, -16.0])
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:the_nether run tp ~ ~ ~",
        );
        assert_eq!(world.teleported.last(), Some(&(HOST, [10.0, 64.0, -2.0])));
        world.poses.push((
            FRIEND,
            Pose {
                feet: [0.0, 0.0, 0.0],
                yaw: 0.0,
                pitch: 0.0,
            },
        ));
        run(&mut server, &mut world, HOST, "tp Alex");
        assert_eq!(
            world.dimension_teleports.last(),
            Some(&(HOST, "minecraft:overworld".to_owned(), [0.0, 0.0, 0.0]))
        );
    }

    #[test]
    fn execute_in_the_nether_asks_for_an_eighth_of_the_overworld_position() {
        let mut server = server();
        let mut world = FakeWorld {
            poses: vec![(
                HOST,
                Pose {
                    feet: [800.0, 70.0, -1600.0],
                    yaw: 0.0,
                    pitch: 0.0,
                },
            )],
            ..FakeWorld::default()
        };
        world.levels.push((NETHER.to_owned(), 8.0));
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute in minecraft:the_nether run tp ~ ~ ~",
        );
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 100.000000, 70.000000, -200.000000"]
        );
        assert_eq!(
            world.dimension_teleports,
            vec![(HOST, NETHER.to_owned(), [100.0, 70.0, -200.0])]
        );
        assert!(world.teleported.is_empty());
    }

    #[test]
    fn execute_positioned_moves_the_source() {
        let mut server = server();
        let mut world = world();
        world.levels.push((NETHER.to_owned(), 8.0));
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "execute positioned 100 70 100 run tp ~ ~ ~",
        );
        assert_eq!(
            texts(&out),
            vec!["Teleported Steve to 100.500000, 70.000000, 100.500000"]
        );
        run(
            &mut server,
            &mut world,
            HOST,
            "execute positioned 800 70 -80 in minecraft:the_nether run tp ~ ~ ~1",
        );
        assert_eq!(
            world.dimension_teleports.last(),
            Some(&(HOST, NETHER.to_owned(), [100.0625, 70.0, -8.9375]))
        );
    }
}
