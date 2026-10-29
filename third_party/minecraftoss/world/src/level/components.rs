use super::Level;
use super::container::Stack;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::{BlockPos, BlockStateId};
use std::collections::BTreeMap;

pub const BANNER: &str = "minecraft:banner";
pub const BANNER_PATTERNS: &str = "minecraft:banner_patterns";
pub const CUSTOM_NAME: &str = "minecraft:custom_name";
pub const NOT_STORED: [&str; 2] = ["minecraft:block_entity_data", "minecraft:block_state"];
pub const BANNER_LOOT: [&str; 5] = [
    CUSTOM_NAME,
    "minecraft:item_name",
    "minecraft:tooltip_display",
    BANNER_PATTERNS,
    "minecraft:rarity",
];

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

pub fn collected(entity: &Tag) -> BTreeMap<String, Tag> {
    let mut collected = entity
        .get("components")
        .and_then(Tag::as_compound)
        .cloned()
        .unwrap_or_default();
    for (component, field) in implicit(kind(entity)) {
        if let Some(value) = entity.get(field) {
            collected.insert((*component).to_owned(), value.clone());
        }
    }
    collected
}

pub fn dropped_components(entity: &Tag) -> Option<Tag> {
    let include: &[&str] = match kind(entity) {
        BANNER => &BANNER_LOOT,
        _ => return None,
    };
    let kept: BTreeMap<String, Tag> = collected(entity)
        .into_iter()
        .filter(|(name, _)| include.contains(&name.as_str()))
        .collect();
    (!kept.is_empty()).then_some(Tag::Compound(kept))
}

pub fn standing(id: &str) -> String {
    id.replacen("_wall_", "_", 1)
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

    pub(super) fn keep_block_entity_components(
        &self,
        pos: BlockPos,
        state: BlockStateId,
        drops: &mut [Stack],
    ) {
        let Some(components) = self.block_entity(pos).and_then(dropped_components) else {
            return;
        };
        let item = standing(self.name(state));
        for stack in drops.iter_mut().filter(|stack| stack.id == item) {
            stack.components = Some(components.clone());
        }
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

    #[test]
    fn a_broken_banner_gives_back_what_its_loot_table_copies() {
        let given = parse(OMINOUS).unwrap();
        let placed = applied(&banner(), &given).unwrap();
        assert_eq!(dropped_components(&placed), Some(given));
        let named = parse(r#"{"minecraft:custom_name":"Flag","minecraft:banner_patterns":[{color:"black",pattern:"minecraft:creeper"}],"minecraft:lore":["Old"],"minecraft:rarity":"rare"}"#).unwrap();
        let placed = applied(&banner(), &named).unwrap();
        let copied = parse(r#"{"minecraft:custom_name":"Flag","minecraft:banner_patterns":[{color:"black",pattern:"minecraft:creeper"}],"minecraft:rarity":"rare"}"#).unwrap();
        assert_eq!(dropped_components(&placed), Some(copied));
        assert_eq!(dropped_components(&banner()), None);
        let spawner =
            parse(r#"{id:"minecraft:mob_spawner",components:{"minecraft:rarity":"rare"}}"#)
                .unwrap();
        assert_eq!(dropped_components(&spawner), None);
        assert_eq!(collected(&spawner).len(), 1);
        assert_eq!(
            standing("minecraft:red_wall_banner"),
            "minecraft:red_banner"
        );
        assert_eq!(standing("minecraft:red_banner"), "minecraft:red_banner");
    }
}
