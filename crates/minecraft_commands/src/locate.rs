use crate::args::{Arg, ArgKind, Registry};
use crate::experience::floor;
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::{Answer, CommandData, Ctx, registered};
use crate::text::{GREEN, Text};
use crate::tree::Node;

pub const STRUCTURE_SEARCH_RADIUS: i32 = 100;
pub const BIOME_SEARCH_RADIUS: i32 = 6400;
pub const BIOME_SAMPLE_HORIZONTAL: i32 = 32;
pub const BIOME_SAMPLE_VERTICAL: i32 = 64;
pub const POI_SEARCH_RADIUS: i32 = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocateKind {
    Structure,
    Biome,
    Poi,
}

impl LocateKind {
    fn registry(self) -> &'static str {
        match self {
            Self::Structure => "minecraft:worldgen/structure",
            Self::Biome => "minecraft:worldgen/biome",
            Self::Poi => "minecraft:point_of_interest_type",
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Structure => "structure",
            Self::Biome => "biome",
            Self::Poi => "poi",
        }
    }

    fn known(
        self,
        data: &CommandData,
    ) -> (&[String], &std::collections::BTreeMap<String, Vec<String>>) {
        match self {
            Self::Structure => (&data.structures, &data.structure_tags),
            Self::Biome => (&data.biomes, &data.biome_tags),
            Self::Poi => (&data.poi_types, &data.poi_tags),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocateRequest {
    pub kind: LocateKind,
    pub printable: String,
    pub tag: bool,
    pub targets: Vec<String>,
    pub dimension: String,
    pub origin: [i32; 3],
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Located {
    pub pos: [i32; 3],
    pub id: String,
}

pub fn distance(request: &LocateRequest, found: &Located) -> i32 {
    let [x, y, z] = request.origin;
    let [fx, fy, fz] = found.pos;
    if request.kind == LocateKind::Biome {
        let squared = [fx - x, fy - y, fz - z]
            .map(|delta| f64::from(delta) * f64::from(delta))
            .iter()
            .sum::<f64>();
        return floor((squared as f32).sqrt());
    }
    let (dx, dz) = (fx.wrapping_sub(x), fz.wrapping_sub(z));
    let squared = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz));
    floor((squared as f32).sqrt())
}

pub fn locate_answer(
    request: &LocateRequest,
    found: Option<&Located>,
) -> Result<(Text, i32), CommandError> {
    let kind = request.kind.name();
    let Some(found) = found else {
        return Err(CommandError::new(
            &format!("commands.locate.{kind}.not_found"),
            vec![literal(&request.printable)],
        ));
    };
    let [x, y, z] = found.pos;
    let height = if request.kind == LocateKind::Biome {
        y.to_string()
    } else {
        "~".to_owned()
    };
    let coordinates = Text::translate(
        "chat.square_brackets",
        vec![Text::translate(
            "chat.coordinates",
            vec![literal(x), literal(height), literal(z)],
        )],
    )
    .color(GREEN);
    let name = if request.tag {
        format!("{} ({})", request.printable, found.id)
    } else {
        request.printable.clone()
    };
    let distance = distance(request, found);
    let text = Text::translate(
        format!("commands.locate.{kind}.success"),
        vec![literal(name), coordinates, literal(distance)],
    );
    Ok((text, distance))
}

pub(crate) fn node() -> Node {
    let search = |kind: &'static str, registry: Registry| {
        Node::literal(kind)
            .then(Node::argument(kind, ArgKind::ResourceOrTag(registry)).executes(locate))
    };
    Node::literal("locate")
        .requires(GAMEMASTERS)
        .then(search("structure", Registry::Structure))
        .then(search("biome", Registry::Biome))
        .then(search("poi", Registry::PointOfInterest))
}

fn locate(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let kind = match ctx.literals.last().copied() {
        Some("biome") => LocateKind::Biome,
        Some("poi") => LocateKind::Poi,
        _ => LocateKind::Structure,
    };
    let Some(Arg::ResourceOrTag { id, tag, end, .. }) = ctx.arg(kind.name()).cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let printable = if tag { format!("#{id}") } else { id.clone() };
    let (ids, tags) = kind.known(ctx.server.data());
    let targets = if tag {
        tags.get(&id).cloned()
    } else {
        registered(ids, &id).then(|| vec![id.clone()])
    };
    let Some(targets) = targets else {
        let registry = literal(kind.registry());
        return Err(match kind {
            LocateKind::Structure => CommandError::new(
                "commands.locate.structure.invalid",
                vec![literal(&printable)],
            ),
            _ if tag => CommandError::new(
                "argument.resource_tag.not_found",
                vec![literal(&id), registry],
            )
            .at(end),
            _ => CommandError::new("argument.resource.not_found", vec![literal(&id), registry])
                .at(end),
        });
    };
    let request = LocateRequest {
        kind,
        printable,
        tag,
        targets,
        dimension: ctx.dimension(),
        origin: ctx.pose().feet.map(|axis| axis.floor() as i32),
    };
    match ctx.world.locate(ctx.source, &request) {
        Answer::Now(found) => {
            let (text, distance) = locate_answer(&request, found.as_ref())?;
            ctx.success(text, false);
            Ok(distance)
        }
        Answer::Later => Ok(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::Recipient;
    use crate::server::Pose;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;

    fn found(id: &str, pos: [i32; 3]) -> Located {
        Located {
            pos,
            id: id.to_owned(),
        }
    }

    fn world() -> FakeWorld {
        FakeWorld {
            poses: vec![(
                HOST,
                Pose {
                    feet: [10.5, 64.0, -3.5],
                    yaw: 0.0,
                    pitch: 0.0,
                },
            )],
            located: vec![
                found("minecraft:village_plains", [100, 0, -50]),
                found("minecraft:cherry_grove", [200, 70, 300]),
            ],
            ..FakeWorld::default()
        }
    }

    fn data() -> CommandData {
        let owned = |ids: &[&str]| ids.iter().map(|id| (*id).to_owned()).collect::<Vec<_>>();
        let mut data = CommandData {
            structures: owned(&[
                "minecraft:village_plains",
                "minecraft:village_desert",
                "minecraft:igloo",
            ]),
            biomes: owned(&["minecraft:cherry_grove", "minecraft:desert"]),
            ..CommandData::default()
        };
        data.structure_tags.insert(
            "minecraft:village".into(),
            owned(&["minecraft:village_desert", "minecraft:village_plains"]),
        );
        data.biome_tags.insert(
            "minecraft:is_forest".into(),
            owned(&["minecraft:cherry_grove"]),
        );
        data
    }

    fn lines(
        server: &mut crate::server::CommandServer,
        world: &mut FakeWorld,
        command: &str,
    ) -> Vec<crate::text::Resolved> {
        server
            .handle(
                HOST,
                crate::protocol::Serverbound::Command(command.into()),
                world,
            )
            .into_iter()
            .map(|out| {
                out.message
                    .chat_text()
                    .unwrap()
                    .resolve(&crate::server::tests::lang)
            })
            .collect()
    }

    #[test]
    fn structures_show_a_horizontal_distance_and_a_tilde_like_vanilla() {
        let mut server = server();
        *server.data_mut() = data();
        let mut world = world();
        let out = lines(
            &mut server,
            &mut world,
            "locate structure minecraft:village_plains",
        );
        assert_eq!(
            out[0].text,
            "The nearest minecraft:village_plains is at [100, ~, -50] (101 blocks away)"
        );
        assert_eq!(out[0].spans, vec![(43..56, GREEN)]);
        assert_eq!(world.located_requests[0].origin, [10, 64, -4]);
        let out = run(&mut server, &mut world, HOST, "locate structure #village");
        assert_eq!(
            texts(&out),
            vec![
                "The nearest #minecraft:village (minecraft:village_plains) is at [100, ~, -50] (101 blocks away)"
            ]
        );
        assert_eq!(
            world.located_requests[1].targets,
            vec!["minecraft:village_desert", "minecraft:village_plains"]
        );
        let out = run(&mut server, &mut world, HOST, "locate structure igloo");
        assert_eq!(
            texts(&out),
            vec!["Could not find a structure of type \"minecraft:igloo\" nearby"]
        );
        assert_eq!(out[0].2, RED);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "locate structure minecraft:nope",
        );
        assert_eq!(
            texts(&out),
            vec!["There is no structure with type \"minecraft:nope\""]
        );
        let out = run(&mut server, &mut world, HOST, "locate structure #nope");
        assert_eq!(
            texts(&out),
            vec!["There is no structure with type \"#minecraft:nope\""]
        );
    }

    #[test]
    fn biomes_show_their_height_and_a_three_dimensional_distance() {
        let mut server = server();
        *server.data_mut() = data();
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "locate biome cherry_grove");
        assert_eq!(
            texts(&out),
            vec!["The nearest minecraft:cherry_grove is at [200, 70, 300] (358 blocks away)"]
        );
        assert_eq!(out[0].0, Recipient::Player(HOST));
        let out = run(&mut server, &mut world, HOST, "locate biome #is_forest");
        assert_eq!(
            texts(&out)[0],
            "The nearest #minecraft:is_forest (minecraft:cherry_grove) is at [200, 70, 300] (358 blocks away)"
        );
        let out = run(&mut server, &mut world, HOST, "locate biome desert");
        assert_eq!(
            texts(&out),
            vec![
                "Could not find a biome of type \"minecraft:desert\" within a reasonable distance"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "locate biome minecraft:nope");
        assert_eq!(
            texts(&out),
            vec![
                "Can't find element 'minecraft:nope' of type 'minecraft:worldgen/biome'",
                "...craft:nope<--[HERE]"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "locate biome #nope");
        assert_eq!(
            texts(&out)[0],
            "Can't find tag 'minecraft:nope' of type 'minecraft:worldgen/biome'"
        );
    }

    #[test]
    fn poi_answers_not_found_without_a_poi_manager() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "locate poi minecraft:home");
        assert_eq!(
            texts(&out),
            vec![
                "Could not find a point of interest of type \"minecraft:home\" within a reasonable distance"
            ]
        );
        world.later = true;
        assert!(
            run(
                &mut server,
                &mut world,
                HOST,
                "locate structure village_plains"
            )
            .is_empty()
        );
        let out = run(
            &mut server,
            &mut world,
            FRIEND,
            "locate structure village_plains",
        );
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }

    #[test]
    fn distances_follow_vanilla_float_rounding() {
        let request = |kind| LocateRequest {
            kind,
            printable: String::new(),
            tag: false,
            targets: Vec::new(),
            dimension: "minecraft:overworld".into(),
            origin: [0, 64, 0],
        };
        let far = found("x", [3, 0, 4]);
        assert_eq!(distance(&request(LocateKind::Structure), &far), 5);
        assert_eq!(distance(&request(LocateKind::Biome), &far), 64);
        assert_eq!(
            distance(&request(LocateKind::Poi), &found("x", [1, 64, 1])),
            1
        );
    }
}
