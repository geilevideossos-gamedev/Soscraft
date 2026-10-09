use crate::args::{Arg, ArgKind, Registry};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::{Ctx, registered};
use crate::text::{GRAY, RED, Text};
use crate::tracker::Tracker;
use crate::tree::Node;

pub const ENCHANTMENT_REGISTRY: &str = "minecraft:enchantment";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnchantmentInfo {
    pub max_level: i32,
    pub curse: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Enchanted {
    Applied,
    Itemless,
    Incompatible(Text),
}

pub fn full_name(id: &str, info: EnchantmentInfo, level: i32) -> Text {
    let (namespace, path) = id.split_once(':').unwrap_or(("minecraft", id));
    let color = if info.curse { RED } else { GRAY };
    let name = Text::translate(format!("enchantment.{namespace}.{path}"), Vec::new()).color(color);
    if level == 1 && info.max_level == 1 {
        return name;
    }
    name.append(Text::literal(" ")).append(Text::translate(
        format!("enchantment.level.{level}"),
        Vec::new(),
    ))
}

pub(crate) fn node() -> Node {
    let level = ArgKind::Int {
        min: 0,
        max: i32::MAX,
    };
    Node::literal("enchant").requires(GAMEMASTERS).then(
        Node::argument("targets", ArgKind::Players { single: false }).then(
            Node::argument("enchantment", ArgKind::Resource(Registry::Enchantment))
                .executes(enchant)
                .then(Node::argument("level", level).executes(enchant)),
        ),
    )
}

fn enchant(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let Some(Arg::Resource { id, at }) = ctx.arg("enchantment").cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let info = registered(&ctx.server.registries().enchantments, &id)
        .then(|| ctx.world.enchantment(&id))
        .flatten();
    let Some(info) = info else {
        return Err(CommandError::new(
            "argument.resource.not_found",
            vec![literal(&id), literal(ENCHANTMENT_REGISTRY)],
        )
        .at(at));
    };
    let targets = ctx.targets("targets")?;
    let level = match ctx.arg("level") {
        Some(Arg::Int(level)) => *level,
        _ => 1,
    };
    if level > info.max_level {
        return Err(CommandError::new(
            "commands.enchant.failed.level",
            vec![literal(level), literal(info.max_level)],
        ));
    }
    let single = targets.len() == 1;
    let mut tracker = Tracker::default();
    for player in targets {
        match ctx.world.enchant(player, &id, level) {
            Some(Enchanted::Applied) => tracker.track(player, 1),
            Some(Enchanted::Incompatible(item)) if single => {
                return Err(CommandError::new(
                    "commands.enchant.failed.incompatible",
                    vec![item],
                ));
            }
            Some(Enchanted::Itemless) if single => {
                return Err(CommandError::new(
                    "commands.enchant.failed.itemless",
                    vec![ctx.display_name(player)],
                ));
            }
            None if single => {
                return Err(CommandError::new(
                    "commands.enchant.failed.entity",
                    vec![ctx.display_name(player)],
                ));
            }
            _ => {}
        }
    }
    if tracker.count(true) == 0 {
        return Err(CommandError::new("commands.enchant.failed", Vec::new()));
    }
    let name = full_name(&id, info, level);
    let text = match tracker.single(true) {
        Some(only) => Text::translate(
            "commands.enchant.success.single",
            vec![name, ctx.display_name(only)],
        ),
        None => Text::translate(
            "commands.enchant.success.multiple",
            vec![name, literal(tracker.count(true))],
        ),
    };
    ctx.success(text, true);
    Ok(tracker.total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::WHITE;

    fn held(id: &str) -> crate::server::tests::Held {
        Some((id.to_owned(), Vec::new()))
    }

    fn world() -> FakeWorld {
        FakeWorld {
            held: vec![(HOST, held("minecraft:diamond_sword"))],
            ..FakeWorld::default()
        }
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
                let lang = crate::server::tests::lang;
                out.message.chat_text().unwrap().resolve(&lang)
            })
            .collect()
    }

    #[test]
    fn enchant_applies_with_the_vanilla_full_name() {
        let mut server = server();
        let mut world = world();
        let out = lines(&mut server, &mut world, "enchant @s sharpness 5");
        assert_eq!(
            out[0].text,
            "Applied enchantment Sharpness V to Steve's item"
        );
        assert_eq!(
            (out[0].color, out[0].spans.clone()),
            (WHITE, vec![(20..31, GRAY)])
        );
        assert_eq!(
            world.held[0].1.as_ref().unwrap().1,
            vec![("minecraft:sharpness".to_owned(), 5)]
        );
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "enchant @s minecraft:mending",
        );
        assert_eq!(
            texts(&out),
            vec!["Applied enchantment Mending to Steve's item"]
        );
        let out = lines(&mut server, &mut world, "enchant Steve binding_curse");
        assert_eq!(
            out[0].text,
            "Applied enchantment Curse of Binding to Steve's item"
        );
        assert_eq!(out[0].spans, vec![(20..36, RED)]);
        let out = run(&mut server, &mut world, HOST, "enchant @s unbreaking");
        assert_eq!(
            texts(&out),
            vec!["Applied enchantment Unbreaking I to Steve's item"]
        );
    }

    #[test]
    fn enchant_fails_in_the_vanilla_order() {
        let mut server = server();
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness 6");
        assert_eq!(
            texts(&out),
            vec!["6 is higher than the maximum level of 5 supported by that enchantment"]
        );
        assert_eq!(out[0].2, RED);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "enchant Herobrine sharpness 6",
        );
        assert_eq!(texts(&out), vec!["No player was found"]);
        run(&mut server, &mut world, HOST, "enchant @s sharpness");
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness 2");
        assert_eq!(
            texts(&out),
            vec!["Diamond Sword cannot support that enchantment"]
        );
        world.held[0].1 = held("minecraft:stick");
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness");
        assert_eq!(texts(&out), vec!["Stick cannot support that enchantment"]);
        world.held[0].1 = None;
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness");
        assert_eq!(texts(&out), vec!["Steve is not holding any item"]);
        let out = run(&mut server, &mut world, HOST, "enchant @s nope");
        assert_eq!(
            texts(&out),
            vec![
                "Can't find element 'minecraft:nope' of type 'minecraft:enchantment'",
                "...nchant @s nope<--[HERE]"
            ]
        );
        let out = run(
            &mut server,
            &mut FakeWorld::default(),
            HOST,
            "enchant @s nope",
        );
        assert_eq!(out.len(), 2);
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness -1");
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn enchant_counts_several_players_and_skips_their_failures() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        world.held.push((FRIEND, None));
        let out = run(&mut server, &mut world, HOST, "enchant @a sharpness 3");
        assert_eq!(
            texts(&out),
            vec!["Applied enchantment Sharpness III to Steve's item"]
        );
        world.held[1].1 = held("minecraft:iron_sword");
        let out = run(&mut server, &mut world, HOST, "enchant @a smite");
        assert_eq!(
            texts(&out),
            vec!["Applied enchantment Smite I to Alex's item"]
        );
        world.held[1].1 = held("minecraft:iron_axe");
        let out = run(&mut server, &mut world, HOST, "enchant @a unbreaking 2");
        assert_eq!(
            texts(&out),
            vec!["Applied enchantment Unbreaking II to 2 entities"]
        );
        let out = run(&mut server, &mut world, HOST, "enchant @a unbreaking 2");
        assert_eq!(
            texts(&out),
            vec![
                "Nothing changed. Targets either have no item in their hands or the enchantment could not be applied"
            ]
        );
        world.held.clear();
        let out = run(&mut server, &mut world, HOST, "enchant @s sharpness");
        assert_eq!(
            texts(&out),
            vec!["Steve is not a valid entity for this command"]
        );
        let out = run(&mut server, &mut world, FRIEND, "enchant @s sharpness");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
    }

    #[test]
    fn full_names_show_the_level_unless_both_are_one() {
        let lang = crate::server::tests::lang;
        let plain = EnchantmentInfo {
            max_level: 1,
            curse: false,
        };
        let five = EnchantmentInfo {
            max_level: 5,
            curse: false,
        };
        let resolve = |text: Text| text.resolve(&lang).text;
        assert_eq!(resolve(full_name("minecraft:mending", plain, 1)), "Mending");
        assert_eq!(resolve(full_name("mending", plain, 2)), "Mending II");
        assert_eq!(
            resolve(full_name("mending", plain, 0)),
            "Mending enchantment.level.0"
        );
        assert_eq!(
            resolve(full_name("minecraft:sharpness", five, 1)),
            "Sharpness I"
        );
        assert_eq!(full_name("minecraft:sharpness", five, 1).color, Some(GRAY));
    }
}
