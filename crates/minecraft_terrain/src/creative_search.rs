pub struct Searched<'a> {
    pub id: &'a str,
    pub lines: &'a [String],
    pub tags: &'a [String],
}

fn split_id(id: &str) -> (&str, &str) {
    id.split_once(':').unwrap_or(("minecraft", id))
}

fn split_query(query: &str) -> Option<(&str, &str)> {
    let at = query.find(':')?;
    Some((query[..at].trim(), query[at + 1..].trim()))
}

fn in_lines(lines: &[String], text: &str) -> bool {
    lines.iter().any(|line| line.to_lowercase().contains(text))
}

pub fn name_matches(query: &str, item: &Searched) -> bool {
    match split_query(query) {
        None => in_lines(item.lines, query),
        Some((namespace, path)) => {
            let (item_namespace, item_path) = split_id(item.id);
            item_namespace.contains(namespace)
                && (item_path.contains(path) || in_lines(item.lines, path))
        }
    }
}

pub fn tag_matches(query: &str, item: &Searched) -> bool {
    match split_query(query) {
        None => item.tags.iter().any(|tag| split_id(tag).1.contains(query)),
        Some((namespace, path)) => {
            item.tags
                .iter()
                .any(|tag| split_id(tag).0.contains(namespace))
                && item.tags.iter().any(|tag| split_id(tag).1.contains(path))
        }
    }
}

pub fn search(query: &str, items: &[Searched]) -> Vec<usize> {
    if query.is_empty() {
        return (0..items.len()).collect();
    }
    let query = query.to_lowercase();
    let matches: Box<dyn Fn(&Searched) -> bool> = match query.strip_prefix('#') {
        Some(tag) => {
            let tag = tag.to_owned();
            Box::new(move |item| tag_matches(&tag, item))
        }
        None => {
            let name = query.clone();
            Box::new(move |item| name_matches(&name, item))
        }
    };
    items
        .iter()
        .enumerate()
        .filter(|(_, item)| matches(item))
        .map(|(index, _)| index)
        .collect()
}

pub fn visible_tags<'a>(query: &str, tags: impl Iterator<Item = &'a str>) -> Vec<String> {
    let Some(query) = query.strip_prefix('#') else {
        return Vec::new();
    };
    let query = query.to_lowercase();
    tags.filter(|tag| {
        let (namespace, path) = split_id(tag);
        match split_query(&query) {
            None => path.contains(query.as_str()),
            Some((wanted_namespace, wanted_path)) => {
                namespace.contains(wanted_namespace) && path.contains(wanted_path)
            }
        }
    })
    .map(str::to_owned)
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Item {
        id: &'static str,
        lines: Vec<String>,
        tags: Vec<String>,
    }

    fn item(id: &'static str, name: &str, tags: &[&str]) -> Item {
        Item {
            id,
            lines: vec![name.to_owned()],
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
        }
    }

    fn items() -> Vec<Item> {
        vec![
            item(
                "minecraft:oak_log",
                "Oak Log",
                &["minecraft:logs", "minecraft:oak_logs"],
            ),
            item("minecraft:white_wool", "White Wool", &["minecraft:wool"]),
            item("minecraft:stone", "Stone", &[]),
            Item {
                id: "minecraft:enchanted_book",
                lines: vec!["Enchanted Book".to_owned(), "Sharpness V".to_owned()],
                tags: Vec::new(),
            },
            item(
                "minecraft:diamond_sword",
                "Diamond Sword",
                &["minecraft:swords"],
            ),
        ]
    }

    fn found(query: &str) -> Vec<&'static str> {
        let items = items();
        let searched: Vec<Searched> = items
            .iter()
            .map(|item| Searched {
                id: item.id,
                lines: &item.lines,
                tags: &item.tags,
            })
            .collect();
        search(query, &searched)
            .into_iter()
            .map(|index| items[index].id)
            .collect()
    }

    #[test]
    fn plain_text_matches_anywhere_in_a_tooltip_line_ignoring_case() {
        assert_eq!(found("").len(), 5);
        assert_eq!(found("wool"), ["minecraft:white_wool"]);
        assert_eq!(found("OOL"), ["minecraft:white_wool"]);
        assert_eq!(
            found("o"),
            [
                "minecraft:oak_log",
                "minecraft:white_wool",
                "minecraft:stone",
                "minecraft:enchanted_book",
                "minecraft:diamond_sword"
            ]
        );
        assert_eq!(found("sharp"), ["minecraft:enchanted_book"]);
        assert_eq!(found("oak_log"), Vec::<&str>::new());
        assert_eq!(found("zzz"), Vec::<&str>::new());
    }

    #[test]
    fn a_colon_searches_the_namespace_and_the_path_or_text() {
        assert_eq!(found("minecraft:oak_log"), ["minecraft:oak_log"]);
        assert_eq!(found("mine : sword"), ["minecraft:diamond_sword"]);
        assert_eq!(found("minecraft:sharpness"), ["minecraft:enchanted_book"]);
        assert_eq!(found("other:stone"), Vec::<&str>::new());
        assert_eq!(found("minecraft:").len(), 5);
    }

    #[test]
    fn a_hash_searches_item_tags() {
        assert_eq!(found("#logs"), ["minecraft:oak_log"]);
        assert_eq!(found("#WOOL"), ["minecraft:white_wool"]);
        assert_eq!(found("#minecraft:sword"), ["minecraft:diamond_sword"]);
        assert_eq!(found("#c:sword"), ["minecraft:diamond_sword"]);
        assert_eq!(found("#other:sword"), Vec::<&str>::new());
        assert_eq!(found("#").len(), 3);
    }

    #[test]
    fn the_shown_tags_follow_the_hash_query() {
        let tags = ["minecraft:logs", "minecraft:oak_logs", "minecraft:wool"];
        assert_eq!(
            visible_tags("#log", tags.iter().copied()),
            ["minecraft:logs", "minecraft:oak_logs"]
        );
        assert_eq!(
            visible_tags("#minecraft:wo", tags.iter().copied()),
            ["minecraft:wool"]
        );
        assert!(visible_tags("log", tags.iter().copied()).is_empty());
    }
}
