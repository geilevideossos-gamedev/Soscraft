use minecraftoss_core::nbt::Tag;
use serde_json::{Map, Value};

pub const WRITABLE_BOOK_CONTENT: &str = "minecraft:writable_book_content";
pub const WRITTEN_BOOK_CONTENT: &str = "minecraft:written_book_content";
pub const CONTENT_COMPONENTS: [&str; 2] = [WRITABLE_BOOK_CONTENT, WRITTEN_BOOK_CONTENT];
pub const WRITABLE_MAX_PAGES: usize = 100;
pub const WRITABLE_PAGE_LENGTH: usize = 1024;
pub const WRITTEN_PAGE_LENGTH: usize = 32767;
pub const TITLE_MAX_LENGTH: usize = 32;
pub const MAX_GENERATION: i64 = 3;

pub fn content(component: &str, snbt: &str) -> Result<Value, String> {
    let tag = minecraftoss_core::snbt::parse_compound(snbt)?;
    match component {
        WRITABLE_BOOK_CONTENT => writable(&tag),
        WRITTEN_BOOK_CONTENT => written(&tag),
        _ => Err(format!("Not a book content component: {component}")),
    }
}

pub fn writable(tag: &Tag) -> Result<Value, String> {
    let mut out = Map::new();
    if let Some(pages) = tag.get("pages") {
        let pages = list(pages, Some(WRITABLE_MAX_PAGES))?
            .iter()
            .map(|page| filterable(page, |value| string(value, WRITABLE_PAGE_LENGTH)))
            .collect::<Result<Vec<_>, _>>()?;
        if !pages.is_empty() {
            out.insert("pages".to_owned(), Value::Array(pages));
        }
    }
    Ok(Value::Object(out))
}

pub fn written(tag: &Tag) -> Result<Value, String> {
    let mut out = Map::new();
    let title = tag.get("title").ok_or("No key title")?;
    out.insert(
        "title".to_owned(),
        filterable(title, |value| string(value, TITLE_MAX_LENGTH))?,
    );
    let author = tag.get("author").ok_or("No key author")?;
    out.insert("author".to_owned(), string(author, usize::MAX)?);
    if let Some(generation) = tag.get("generation") {
        let generation = number(generation)?;
        if !(0..=MAX_GENERATION).contains(&generation) {
            return Err(format!(
                "Value {generation} outside of range [0:{MAX_GENERATION}]"
            ));
        }
        if generation != 0 {
            out.insert("generation".to_owned(), Value::from(generation));
        }
    }
    if let Some(pages) = tag.get("pages") {
        let pages = list(pages, None)?
            .iter()
            .map(|page| filterable(page, component))
            .collect::<Result<Vec<_>, _>>()?;
        if !pages.is_empty() {
            out.insert("pages".to_owned(), Value::Array(pages));
        }
    }
    if let Some(resolved) = tag.get("resolved")
        && number(resolved)? != 0
    {
        out.insert("resolved".to_owned(), Value::Bool(true));
    }
    Ok(Value::Object(out))
}

pub fn pages(components: &Value) -> Option<&Vec<Value>> {
    CONTENT_COMPONENTS
        .iter()
        .find_map(|component| components.get(component))
        .map(|content| content.get("pages").and_then(Value::as_array))
        .map(|pages| pages.unwrap_or(&EMPTY))
}

static EMPTY: Vec<Value> = Vec::new();

pub fn page_count(components: Option<&Value>) -> usize {
    components.and_then(pages).map_or(0, Vec::len)
}

pub fn has_content(id: &str, components: Option<&Value>) -> bool {
    id == "minecraft:writable_book"
        || components.is_some_and(|components| components.get(WRITTEN_BOOK_CONTENT).is_some())
}

pub fn raw_page(page: &Value) -> &Value {
    page.get("raw").unwrap_or(page)
}

pub fn split_entries(spec: &str) -> Vec<&str> {
    let mut entries = Vec::new();
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    let mut start = 0;
    for (at, c) in spec.char_indices() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => depth -= 1,
                ',' if depth <= 0 => {
                    entries.push(&spec[start..at]);
                    start = at + 1;
                }
                _ => {}
            },
        }
    }
    entries.push(&spec[start..]);
    entries
}

pub fn entry_count(entry: &str) -> Option<(&str, u32)> {
    let tail_from = match component_list_start(entry) {
        Some(open) => open + closing(&entry[open..])?,
        None => 0,
    };
    match entry[tail_from..].find('*') {
        Some(star) => {
            let star = tail_from + star;
            let count = entry[star + 1..].trim().parse::<u32>().ok()?;
            Some((entry[..star].trim(), count))
        }
        None => Some((entry.trim(), 1)),
    }
}

pub fn component_list_start(entry: &str) -> Option<usize> {
    entry.find('[')
}

fn closing(list: &str) -> Option<usize> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    for (at, c) in list.char_indices() {
        match quote {
            Some(_) if escaped => escaped = false,
            Some(_) if c == '\\' => escaped = true,
            Some(open) if c == open => quote = None,
            Some(_) => {}
            None => match c {
                '"' | '\'' => quote = Some(c),
                '[' | '{' => depth += 1,
                ']' | '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(at + 1);
                    }
                }
                _ => {}
            },
        }
    }
    None
}

pub(crate) fn describe(tag: &Tag) -> String {
    tag_json(tag).to_string()
}

pub(crate) fn list(tag: &Tag, max: Option<usize>) -> Result<&Vec<Tag>, String> {
    let Tag::List(items) = tag else {
        return Err(format!("Not a list: {}", describe(tag)));
    };
    if let Some(max) = max.filter(|max| items.len() > *max) {
        return Err(format!(
            "List is too long: {}, expected range [0-{max}]",
            items.len()
        ));
    }
    Ok(items)
}

pub(crate) fn string(tag: &Tag, max: usize) -> Result<Value, String> {
    let Tag::String(text) = tag else {
        return Err(format!("Not a string: {}", describe(tag)));
    };
    let length = text.chars().count();
    if length > max {
        return Err(format!("String \"{text}\" is too long: {length} > {max}"));
    }
    Ok(Value::from(text.clone()))
}

pub(crate) fn number(tag: &Tag) -> Result<i64, String> {
    match tag {
        Tag::Byte(v) => Ok(i64::from(*v)),
        Tag::Short(v) => Ok(i64::from(*v)),
        Tag::Int(v) => Ok(i64::from(*v)),
        Tag::Long(v) => Ok(*v),
        _ => Err(format!("Not a number: {}", describe(tag))),
    }
}

fn component(tag: &Tag) -> Result<Value, String> {
    match tag {
        Tag::String(_) => string(tag, WRITTEN_PAGE_LENGTH),
        Tag::Compound(_) | Tag::List(_) => Ok(tag_json(tag)),
        _ => Err(format!("Not a text component: {}", describe(tag))),
    }
}

fn filterable(tag: &Tag, inner: impl Fn(&Tag) -> Result<Value, String>) -> Result<Value, String> {
    let mut out = Map::new();
    match tag.get("raw") {
        Some(raw) => {
            out.insert("raw".to_owned(), inner(raw)?);
            if let Some(filtered) = tag.get("filtered") {
                out.insert("filtered".to_owned(), inner(filtered)?);
            }
        }
        None => {
            out.insert("raw".to_owned(), inner(tag)?);
        }
    }
    Ok(Value::Object(out))
}

pub(crate) fn tag_json(tag: &Tag) -> Value {
    match tag {
        Tag::Byte(v) => Value::from(*v),
        Tag::Short(v) => Value::from(*v),
        Tag::Int(v) => Value::from(*v),
        Tag::Long(v) => Value::from(*v),
        Tag::Float(v) => Value::from(f64::from(*v)),
        Tag::Double(v) => Value::from(*v),
        Tag::ByteArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::String(s) => Value::from(s.clone()),
        Tag::List(items) => Value::Array(items.iter().map(tag_json).collect()),
        Tag::Compound(map) => Value::Object(
            map.iter()
                .map(|(key, value)| (key.clone(), tag_json(value)))
                .collect(),
        ),
        Tag::IntArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
        Tag::LongArray(a) => Value::Array(a.iter().map(|&v| Value::from(v)).collect()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_written_book_keeps_title_author_and_pages_in_the_saved_shape() {
        let content = content(
            WRITTEN_BOOK_CONTENT,
            r#"{title:"Test",author:"Owner",pages:["One, two","Three",{text:"Four",color:"red"},{raw:"Five",filtered:"5"}]}"#,
        )
        .unwrap();
        assert_eq!(
            content,
            json!({
                "title": {"raw": "Test"},
                "author": "Owner",
                "pages": [{"raw": "One, two"}, {"raw": "Three"}, {"raw": {"text": "Four", "color": "red"}}, {"raw": "Five", "filtered": "5"}]
            })
        );
        let copied = content_of(r#"{title:{raw:"T"},author:"A",generation:2,resolved:1b}"#);
        assert_eq!(
            copied,
            json!({"title": {"raw": "T"}, "author": "A", "generation": 2, "resolved": true})
        );
    }

    fn content_of(snbt: &str) -> Value {
        content(WRITTEN_BOOK_CONTENT, snbt).unwrap()
    }

    #[test]
    fn writable_pages_are_plain_or_raw_strings() {
        let content = content(
            WRITABLE_BOOK_CONTENT,
            r#"{pages:["a",{raw:"b"},{raw:"c",filtered:"d"}]}"#,
        )
        .unwrap();
        assert_eq!(
            content,
            json!({"pages": [{"raw": "a"}, {"raw": "b"}, {"raw": "c", "filtered": "d"}]})
        );
        assert_eq!(
            super::content(WRITABLE_BOOK_CONTENT, "{}").unwrap(),
            json!({})
        );
    }

    #[test]
    fn bad_book_content_is_refused() {
        let refused = |component: &str, snbt: &str| content(component, snbt).unwrap_err();
        assert_eq!(
            refused(WRITTEN_BOOK_CONTENT, r#"{author:"A"}"#),
            "No key title"
        );
        assert_eq!(
            refused(WRITTEN_BOOK_CONTENT, r#"{title:"T"}"#),
            "No key author"
        );
        assert!(
            refused(
                WRITTEN_BOOK_CONTENT,
                &format!(r#"{{title:"{}",author:"A"}}"#, "x".repeat(33))
            )
            .contains("too long")
        );
        assert!(
            refused(
                WRITTEN_BOOK_CONTENT,
                r#"{title:"T",author:"A",generation:4}"#
            )
            .contains("outside of range")
        );
        assert!(
            refused(
                WRITTEN_BOOK_CONTENT,
                r#"{title:"T",author:"A",pages:"one"}"#
            )
            .starts_with("Not a list")
        );
        assert!(
            refused(WRITTEN_BOOK_CONTENT, r#"{title:"T",author:"A",pages:[1]}"#)
                .starts_with("Not a text component")
        );
        assert!(
            refused(WRITABLE_BOOK_CONTENT, r#"{pages:[{text:"a"}]}"#).starts_with("Not a string")
        );
        let many = vec!["\"p\""; 101].join(",");
        assert!(
            refused(WRITABLE_BOOK_CONTENT, &format!("{{pages:[{many}]}}"))
                .starts_with("List is too long")
        );
        assert!(refused(WRITABLE_BOOK_CONTENT, r#"{pages:["a"]"#).contains("at position"));
    }

    #[test]
    fn page_counts_and_content_follow_the_components() {
        let written = json!({WRITTEN_BOOK_CONTENT: content_of(r#"{title:"T",author:"A",pages:["a","b","c"]}"#)});
        assert_eq!(page_count(Some(&written)), 3);
        assert!(has_content("minecraft:written_book", Some(&written)));
        assert!(!has_content("minecraft:written_book", None));
        assert!(has_content("minecraft:writable_book", None));
        assert_eq!(page_count(None), 0);
        let empty = json!({WRITABLE_BOOK_CONTENT: {}});
        assert_eq!(page_count(Some(&empty)), 0);
        assert_eq!(raw_page(&json!({"raw": "x"})), &json!("x"));
        assert_eq!(raw_page(&json!("y")), &json!("y"));
    }

    #[test]
    fn kit_entries_split_outside_components_and_quotes() {
        let spec = r#"ice*2,written_book[written_book_content={title:"a,b",author:"c",pages:["x, *3",'y]']}]*2,stone"#;
        let entries = split_entries(spec);
        assert_eq!(entries.len(), 3, "{entries:?}");
        assert_eq!(entry_count(entries[0]), Some(("ice", 2)));
        assert_eq!(
            entry_count(entries[1]),
            Some((&entries[1][..entries[1].len() - 2], 2))
        );
        assert_eq!(entry_count(entries[2]), Some(("stone", 1)));
        assert_eq!(
            entry_count("book[x={a:\"*\"}]"),
            Some(("book[x={a:\"*\"}]", 1))
        );
        assert_eq!(entry_count("book[x={a:1}"), None);
        assert_eq!(entry_count("ice*many"), None);
    }
}
