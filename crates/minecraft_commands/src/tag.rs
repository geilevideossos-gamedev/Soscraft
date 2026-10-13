use crate::args::{Arg, ArgKind};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, literal};
use crate::server::Ctx;
use crate::text::{GREEN, Text};
use crate::tracker::Tracker;
use crate::tree::Node;

pub const MAX_TAGS: usize = 1024;

pub fn add(tags: &mut Vec<String>, tag: &str) -> bool {
    if tags.len() >= MAX_TAGS || tags.iter().any(|known| known == tag) {
        return false;
    }
    tags.push(tag.to_owned());
    true
}

pub fn remove(tags: &mut Vec<String>, tag: &str) -> bool {
    let before = tags.len();
    tags.retain(|known| known != tag);
    tags.len() != before
}

pub(crate) fn node() -> Node {
    let name = || Node::argument("name", ArgKind::Word);
    Node::literal("tag").requires(GAMEMASTERS).then(
        Node::argument("targets", ArgKind::Players { single: false })
            .then(Node::literal("add").then(name().executes(add_tag)))
            .then(Node::literal("remove").then(name().executes(remove_tag)))
            .then(Node::literal("list").executes(list_tags)),
    )
}

fn name(ctx: &Ctx) -> String {
    match ctx.arg("name") {
        Some(Arg::Text(name)) => name.clone(),
        _ => String::new(),
    }
}

fn add_tag(ctx: &mut Ctx) -> Result<i32, CommandError> {
    change(ctx, true)
}

fn remove_tag(ctx: &mut Ctx) -> Result<i32, CommandError> {
    change(ctx, false)
}

fn change(ctx: &mut Ctx, adding: bool) -> Result<i32, CommandError> {
    let targets = ctx.targets("targets")?;
    let tag = name(ctx);
    let mut tracker = Tracker::default();
    for player in targets {
        let changed = if adding {
            ctx.world.add_tag(player, &tag)
        } else {
            ctx.world.remove_tag(player, &tag)
        };
        tracker.track(player, i32::from(changed));
    }
    let (failed, single, multiple) = if adding {
        (
            "commands.tag.add.failed",
            "commands.tag.add.success.single",
            "commands.tag.add.success.multiple",
        )
    } else {
        (
            "commands.tag.remove.failed",
            "commands.tag.remove.success.single",
            "commands.tag.remove.success.multiple",
        )
    };
    if tracker.count(true) == 0 {
        return Err(CommandError::new(failed, Vec::new()));
    }
    let text = match tracker.single(true) {
        Some(only) => Text::translate(single, vec![literal(&tag), ctx.display_name(only)]),
        None => Text::translate(multiple, vec![literal(&tag), literal(tracker.count(true))]),
    };
    ctx.success(text, true);
    Ok(tracker.total)
}

fn list_tags(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let targets = ctx.targets("targets")?;
    let mut tracker = Tracker::default();
    let mut all: Vec<String> = Vec::new();
    for player in targets {
        let tags: Vec<String> = ctx.world.tags(player).into_iter().flatten().collect();
        for tag in &tags {
            if !all.contains(tag) {
                all.push(tag.clone());
            }
        }
        tracker.track(player, i32::try_from(tags.len()).unwrap_or(i32::MAX));
    }
    all.sort();
    let text = match (all.is_empty(), tracker.single(false)) {
        (true, Some(only)) => Text::translate(
            "commands.tag.list.single.empty",
            vec![ctx.display_name(only)],
        ),
        (true, None) => Text::translate(
            "commands.tag.list.multiple.empty",
            vec![literal(tracker.count(false))],
        ),
        (false, Some(only)) => Text::translate(
            "commands.tag.list.single.success",
            vec![
                ctx.display_name(only),
                literal(all.len()),
                format_list(&all),
            ],
        ),
        (false, None) => Text::translate(
            "commands.tag.list.multiple.success",
            vec![
                literal(tracker.count(false)),
                literal(all.len()),
                format_list(&all),
            ],
        ),
    };
    ctx.success(text, false);
    Ok(i32::try_from(all.len()).unwrap_or(i32::MAX))
}

fn format_list(tags: &[String]) -> Text {
    let mut text = Text::literal("");
    for (at, tag) in tags.iter().enumerate() {
        if at > 0 {
            text = text.append(Text::literal(", "));
        }
        text = text.append(Text::literal(tag.clone()).color(GREEN));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};

    fn world() -> FakeWorld {
        FakeWorld {
            tags: vec![
                (HOST, Vec::new()),
                (
                    FRIEND,
                    vec!["builder".to_owned(), "iw4l_loadout_given".to_owned()],
                ),
            ],
            ..FakeWorld::default()
        }
    }

    #[test]
    fn add_and_remove_report_like_vanilla() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "tag @s add iw4l_loadout_given",
        );
        assert_eq!(texts(&out), vec!["Added tag 'iw4l_loadout_given' to Steve"]);
        assert_eq!(world.tags[0].1, vec!["iw4l_loadout_given"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "tag @s add iw4l_loadout_given",
        );
        assert_eq!(
            texts(&out),
            vec!["Target either already has the tag or has too many tags"]
        );
        let out = run(&mut server, &mut world, HOST, "tag @a add red");
        assert_eq!(texts(&out), vec!["Added tag 'red' to 2 entities"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "tag @a remove iw4l_loadout_given",
        );
        assert_eq!(
            texts(&out),
            vec!["Removed tag 'iw4l_loadout_given' from 2 entities"]
        );
        assert_eq!(world.tags[1].1, vec!["builder", "red"]);
        let out = run(&mut server, &mut world, HOST, "tag Alex remove builder");
        assert_eq!(texts(&out), vec!["Removed tag 'builder' from Alex"]);
        let out = run(&mut server, &mut world, HOST, "tag Alex remove builder");
        assert_eq!(texts(&out), vec!["Target does not have this tag"]);
    }

    #[test]
    fn list_sorts_the_tags_of_every_target() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        let out = run(&mut server, &mut world, HOST, "tag @s list");
        assert_eq!(texts(&out), vec!["Steve has no tags"]);
        let out = run(&mut server, &mut world, HOST, "tag Alex list");
        assert_eq!(
            texts(&out),
            vec!["Alex has 2 tag(s): builder, iw4l_loadout_given"]
        );
        run(&mut server, &mut world, HOST, "tag @s add alpha");
        let out = run(&mut server, &mut world, HOST, "tag @a list");
        assert_eq!(
            texts(&out),
            vec!["The 2 entities have 3 total tag(s): alpha, builder, iw4l_loadout_given"]
        );
        run(&mut server, &mut world, HOST, "tag @a remove alpha");
        run(&mut server, &mut world, HOST, "tag @a remove builder");
        run(
            &mut server,
            &mut world,
            HOST,
            "tag @a remove iw4l_loadout_given",
        );
        let out = run(&mut server, &mut world, HOST, "tag @a list");
        assert_eq!(texts(&out), vec!["There are no tags on the 2 entities"]);
    }

    #[test]
    fn tags_are_unique_and_capped() {
        let mut tags = Vec::new();
        assert!(add(&mut tags, "a"));
        assert!(!add(&mut tags, "a"));
        for at in 1..MAX_TAGS {
            assert!(add(&mut tags, &format!("t{at}")));
        }
        assert!(!add(&mut tags, "one_more"));
        assert!(remove(&mut tags, "a"));
        assert!(!remove(&mut tags, "a"));
        assert!(add(&mut tags, "one_more"));
    }

    #[test]
    fn only_game_masters_may_tag() {
        let mut server = server();
        server.join(FRIEND, "Alex");
        let mut world = world();
        let out = run(&mut server, &mut world, FRIEND, "tag @s add x");
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        assert_eq!(world.tags[1].1.len(), 2);
    }
}
