use std::path::Path;

use crate::args::{Arg, ArgKind, ItemArg};
use crate::permissions::OWNERS;
use crate::reader::{CommandError, StringReader, literal};
use crate::server::{Ctx, error_line, registered};
use crate::text::Text;
use crate::tree::Node;

pub const KIT_KEY: &str = "IW4L_MINECRAFT_KIT";

pub fn env_value(text: &str, key: &str) -> Option<String> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let line = line.strip_prefix("export ").unwrap_or(line);
        let (name, value) = line.split_once('=')?;
        if line.starts_with('#') || name.trim() != key {
            return None;
        }
        let value = value.trim();
        let unquoted = ['"', '\'']
            .iter()
            .find_map(|quote| value.strip_prefix(*quote)?.strip_suffix(*quote));
        let value = match unquoted {
            Some(inner) => inner,
            None => value.split(" #").next().unwrap_or(value).trim_end(),
        };
        Some(value.to_owned())
    })
}

pub fn read_env_file(path: &Path, key: &str) -> Option<String> {
    env_value(&std::fs::read_to_string(path).ok()?, key)
}

pub fn kit_entries(spec: &str) -> Vec<(ItemArg, u32)> {
    minecraftoss_world::book::split_entries(spec)
        .into_iter()
        .filter_map(|entry| {
            let (item, count) = minecraftoss_world::book::entry_count(entry.trim())?;
            if item.is_empty() || count == 0 {
                return None;
            }
            let mut reader = StringReader::new(item);
            match ArgKind::Item.parse(&mut reader, &[]) {
                Ok(Arg::Item(item)) if !reader.can_read() && !item.id.ends_with(':') => {
                    Some((item, count))
                }
                _ => None,
            }
        })
        .collect()
}

pub(crate) fn node() -> Node {
    Node::literal("kit")
        .requires(OWNERS)
        .executes(kit)
        .then(Node::argument("targets", ArgKind::Players { single: false }).executes(kit))
}

fn kit(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let targets = ctx.targets("targets")?;
    let Some(line) = ctx.world.kit_line() else {
        return Err(CommandError {
            text: Text::literal(format!("No {KIT_KEY} in the .env next to the game")),
            cursor: None,
        });
    };
    let entries = ctx.world.kit_entries(&line);
    if entries.is_empty() {
        return Err(CommandError {
            text: Text::literal(format!("{KIT_KEY} has no items")),
            cursor: None,
        });
    }
    let mut given = 0;
    for (item, count) in entries {
        if !registered(&ctx.server.registries().items, &item.id) {
            let unknown = Text::translate("argument.item.id.invalid", vec![literal(&item.id)]);
            ctx.out.push(error_line(ctx.source, unknown));
            continue;
        }
        let reached: Vec<_> = targets
            .iter()
            .copied()
            .filter(|player| ctx.world.give(*player, &item, count))
            .collect();
        let text = match reached.as_slice() {
            [] => continue,
            [only] => Text::translate(
                "commands.give.success.single",
                vec![
                    literal(count),
                    Text::item(&item.id),
                    ctx.display_name(*only),
                ],
            ),
            _ => Text::translate(
                "commands.give.success.multiple",
                vec![literal(count), Text::item(&item.id), literal(reached.len())],
            ),
        };
        given += 1;
        ctx.success(text, true);
    }
    Ok(given)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server::tests::{FRIEND, FakeWorld, HOST, run, server, texts};
    use crate::text::RED;

    #[test]
    fn env_files_are_read_like_dotenv() {
        let text = "A=1\r\n# IW4L_MINECRAFT_KIT=commented\r\nexport IW4L_MINECRAFT_KIT = \"stone*64, torch\" \r\nIW4L_MINECRAFT_KIT=later\r\n";
        assert_eq!(env_value(text, KIT_KEY).as_deref(), Some("stone*64, torch"));
        assert_eq!(
            env_value("IW4L_MINECRAFT_KIT='a,b'", KIT_KEY).as_deref(),
            Some("a,b")
        );
        assert_eq!(
            env_value("IW4L_MINECRAFT_KIT=stone*64 # comment", KIT_KEY).as_deref(),
            Some("stone*64")
        );
        assert_eq!(
            env_value("IW4L_MINECRAFT_KIT=", KIT_KEY).as_deref(),
            Some("")
        );
        assert_eq!(env_value("OTHER=1", KIT_KEY), None);
        let dir =
            std::env::temp_dir().join(format!("minecraft_commands-kit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(".env");
        assert_eq!(read_env_file(&path, KIT_KEY), None);
        std::fs::write(&path, "IW4L_MINECRAFT_KIT=stone").unwrap();
        assert_eq!(read_env_file(&path, KIT_KEY).as_deref(), Some("stone"));
        std::fs::write(&path, "IW4L_MINECRAFT_KIT=torch*8").unwrap();
        assert_eq!(read_env_file(&path, KIT_KEY).as_deref(), Some("torch*8"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn kit_entries_take_counts_and_enchantments() {
        let entries = kit_entries(
            "diamond_pickaxe+efficiency=5, stone * 64,minecraft:torch*2,bad*0,*,x*y,Stone,,mod:thing",
        );
        let summary: Vec<_> = entries
            .iter()
            .map(|(item, count)| (item.id.as_str(), item.enchantments.len(), *count))
            .collect();
        assert_eq!(
            summary,
            vec![
                ("minecraft:diamond_pickaxe", 1, 1),
                ("minecraft:stone", 0, 64),
                ("minecraft:torch", 0, 2),
                ("mod:thing", 0, 1)
            ]
        );
        assert_eq!(
            entries[0].0.enchantments,
            vec![("minecraft:efficiency".to_owned(), 5)]
        );
    }

    #[test]
    fn kit_entries_keep_commas_inside_components() {
        let entries = kit_entries(
            r#"written_book[written_book_content={title:"a,b",author:"c",pages:["x, y*2","z"]}]*3,stone"#,
        );
        assert_eq!(entries.len(), 2);
        let (book, count) = &entries[0];
        assert_eq!((book.id.as_str(), *count), ("minecraft:written_book", 3));
        assert_eq!(
            book.components,
            Some(serde_json::json!({"minecraft:written_book_content": {
                "title": {"raw": "a,b"},
                "author": "c",
                "pages": [{"raw": "x, y*2"}, {"raw": "z"}]
            }}))
        );
        assert_eq!(entries[1].0.id, "minecraft:stone");
    }

    #[test]
    fn kit_gives_every_entry_and_needs_level_four() {
        let mut server = server();
        server.registries_mut().items =
            vec!["minecraft:stone".into(), "minecraft:diamond_sword".into()];
        server.join(FRIEND, "Alex");
        let mut world = FakeWorld {
            kit: Some("stone*64,diamond_sword+sharpness=3,notanitem".into()),
            ..FakeWorld::default()
        };
        let out = run(&mut server, &mut world, HOST, "kit");
        assert_eq!(
            texts(&out),
            vec![
                "Gave 64 [Stone] to Steve",
                "Gave 1 [Diamond Sword] to Steve",
                "Unknown item 'minecraft:notanitem'"
            ]
        );
        assert_eq!(out[2].2, RED);
        assert_eq!(world.given.len(), 2);
        assert_eq!(
            world.given[1].1.enchantments,
            vec![("minecraft:sharpness".to_owned(), 3)]
        );
        let out = run(&mut server, &mut world, HOST, "kit @a");
        assert_eq!(texts(&out)[0], "Gave 64 [Stone] to 2 players");
        let dir =
            std::env::temp_dir().join(format!("minecraft_commands-kit-ops-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(crate::permissions::OPS_FILE);
        let mut ops = crate::permissions::OpList::default();
        ops.add(&crate::offline_uuid("Alex"), "Alex", 3);
        ops.save(&path).unwrap();
        server.open_ops(&path).unwrap();
        assert_eq!(server.level(FRIEND), 3);
        let out = run(&mut server, &mut world, FRIEND, "kit");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            texts(&out)[0],
            "Unknown or incomplete command. See below for error"
        );
        world.kit = Some(" , ".into());
        let out = run(&mut server, &mut world, HOST, "kit");
        assert_eq!(texts(&out), vec!["IW4L_MINECRAFT_KIT has no items"]);
        world.kit = None;
        let out = run(&mut server, &mut world, HOST, "kit");
        assert_eq!(
            texts(&out),
            vec!["No IW4L_MINECRAFT_KIT in the .env next to the game"]
        );
    }
}
