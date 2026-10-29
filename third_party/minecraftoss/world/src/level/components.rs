use super::Level;
use minecraftoss_core::BlockPos;
use minecraftoss_core::nbt::Tag;
use std::collections::BTreeMap;

pub const BANNER: &str = "minecraft:banner";
pub const BANNER_PATTERNS: &str = "minecraft:banner_patterns";
pub const CUSTOM_NAME: &str = "minecraft:custom_name";
pub const NOT_STORED: [&str; 2] = ["minecraft:block_entity_data", "minecraft:block_state"];

pub fn is_banner(id: &str) -> bool {
    id.strip_prefix("minecraft:")
        .is_some_and(|path| path.ends_with("_banner"))
}

pub fn implicit(kind: &str) -> &'static [(&'static str, &'static str)] {
    match kind {
        BANNER => &[(BANNER_PATTERNS, "patterns"), (CUSTOM_NAME, "CustomName")],
        _ => &[],
    }
}

pub fn kind(entity: &Tag) -> &str {
    entity.get("id").and_then(Tag::as_str).unwrap_or_default()
}

pub fn applied(entity: &Tag, components: &Tag) -> Option<Tag> {
    let mut applied = entity.as_compound()?.clone();
    let implicit = implicit(kind(entity));
    for (_, field) in implicit {
        applied.remove(*field);
    }
    let mut stored = BTreeMap::new();
    for (name, value) in components.as_compound().into_iter().flatten() {
        if name.starts_with('!') || NOT_STORED.contains(&name.as_str()) {
            continue;
        }
        match implicit.iter().find(|(component, _)| component == name) {
            Some((_, field)) if value.as_list().is_none_or(|list| !list.is_empty()) => {
                applied.insert((*field).to_owned(), value.clone());
            }
            Some(_) => {}
            None => {
                stored.insert(name.clone(), value.clone());
            }
        }
    }
    applied.insert("components".to_owned(), Tag::Compound(stored));
    Some(Tag::Compound(applied))
}

impl Level<'_> {
    pub fn apply_block_entity_components(&mut self, pos: BlockPos, components: &Tag) -> bool {
        let state = self.block(pos);
        if self.block_entity(pos).is_none() {
            self.ensure_block_entity(pos, state);
        }
        let Some(entity) = self.block_entity(pos) else {
            return false;
        };
        let Some(tag) = applied(entity, components) else {
            return false;
        };
        self.set_block_entity_tag(pos, tag);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::snbt::parse;

    const OMINOUS: &str = r#"{"minecraft:banner_patterns":[{color:"cyan",pattern:"minecraft:rhombus"},{color:"light_gray",pattern:"minecraft:stripe_bottom"}],"minecraft:item_name":{translate:"block.minecraft.ominous_banner"},"minecraft:rarity":"uncommon","minecraft:tooltip_display":{hidden_components:["minecraft:banner_patterns"]}}"#;

    fn banner() -> Tag {
        parse(r#"{id:"minecraft:banner",x:1,y:2,z:3,components:{}}"#).unwrap()
    }

    #[test]
    fn a_banner_takes_its_patterns_and_name_and_stores_the_other_components() {
        let given = parse(OMINOUS).unwrap();
        let tag = applied(&banner(), &given).unwrap();
        assert_eq!(tag.get("patterns"), given.get(BANNER_PATTERNS));
        assert_eq!(tag.get("CustomName"), None);
        assert_eq!((kind(&tag), tag.get("x")), (BANNER, Some(&Tag::Int(1))));
        let stored = tag.get("components").and_then(Tag::as_compound).unwrap();
        assert_eq!(
            stored.keys().map(String::as_str).collect::<Vec<_>>(),
            [
                "minecraft:item_name",
                "minecraft:rarity",
                "minecraft:tooltip_display"
            ]
        );
        assert_eq!(
            stored.get("minecraft:rarity"),
            given.get("minecraft:rarity")
        );
        let named = parse(
            r#"{"minecraft:custom_name":"Flag","minecraft:block_state":{rotation:"4"},"minecraft:block_entity_data":{id:"minecraft:banner"},"!minecraft:food":{}}"#,
        )
        .unwrap();
        let tag = applied(&banner(), &named).unwrap();
        assert_eq!(tag.get("CustomName"), Some(&Tag::String("Flag".to_owned())));
        assert_eq!(tag.get("patterns"), None);
        assert_eq!(tag.get("components"), Some(&Tag::Compound(BTreeMap::new())));
    }

    #[test]
    fn placing_again_replaces_the_patterns_the_name_and_the_components() {
        let before = applied(&banner(), &parse(OMINOUS).unwrap()).unwrap();
        let named = applied(
            &before,
            &parse(r#"{"minecraft:custom_name":"Flag"}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(named.get("patterns"), None);
        assert_eq!(
            named.get("CustomName"),
            Some(&Tag::String("Flag".to_owned()))
        );
        assert_eq!(
            named.get("components"),
            Some(&Tag::Compound(BTreeMap::new()))
        );
        let empty = applied(
            &named,
            &parse(r#"{"minecraft:banner_patterns":[]}"#).unwrap(),
        )
        .unwrap();
        assert_eq!(
            (empty.get("patterns"), empty.get("CustomName")),
            (None, None)
        );
    }

    #[test]
    fn other_block_entities_store_every_component() {
        let spawner = parse(r#"{id:"minecraft:mob_spawner",components:{}}"#).unwrap();
        let given = parse(r#"{"minecraft:custom_name":"Cage","minecraft:rarity":"rare"}"#).unwrap();
        let tag = applied(&spawner, &given).unwrap();
        assert_eq!(tag.get("CustomName"), None);
        assert_eq!(tag.get("components"), Some(&given));
        assert_eq!(applied(&Tag::Int(1), &given), None);
        assert!(is_banner("minecraft:white_banner") && is_banner("minecraft:red_wall_banner"));
        assert!(!is_banner("minecraft:shield") && !is_banner("other:white_banner"));
    }
}
