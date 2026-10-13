use minecraftoss_player::inventory::ItemStack;
use serde_json::{Map, Value};

use crate::fork_items::{self, CUSTOM_DATA, FULL_CLIP, Kind};

pub const AKIMBO: &str = "akimbo";
pub const CLIP_KEYS: [&str; 3] = ["clip", "clip_left", "alt_clip"];

pub trait GunRules {
    fn accepts(&self, id: &str, attachments: &[String]) -> bool;
    fn clip_size(&self, id: &str, attachments: &[String]) -> Option<i32>;
}

#[derive(Clone, Debug, PartialEq)]
pub struct Refit {
    pub stack: ItemStack,
    pub rounds: u32,
}

pub fn is_gun(stack: &ItemStack) -> bool {
    fork_items::get(&stack.id).is_some_and(|row| row.kind == Kind::Gun)
}

fn state(stack: &ItemStack) -> Option<&Map<String, Value>> {
    stack
        .components
        .as_ref()?
        .get(CUSTOM_DATA)?
        .get("iw4l")?
        .as_object()
}

pub fn attachments(stack: &ItemStack) -> Vec<String> {
    state(stack)
        .and_then(|state| state.get("attachments"))
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

pub fn clip(stack: &ItemStack, key: &str) -> Option<i64> {
    state(stack)?.get(key)?.as_i64()
}

pub fn clips(stack: &ItemStack) -> [Option<i64>; 3] {
    CLIP_KEYS.map(|key| clip(stack, key))
}

pub fn with_state(stack: &ItemStack, attachments: &[String], clips: [Option<i64>; 3]) -> ItemStack {
    let mut stack = ItemStack {
        count: 1,
        ..stack.clone()
    };
    let mut components = match stack.components.take() {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    let mut custom = match components.remove(CUSTOM_DATA) {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    let mut state = match custom.remove("iw4l") {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    };
    if attachments.is_empty() {
        state.remove("attachments");
    } else {
        state.insert("attachments".to_owned(), Value::from(attachments.to_vec()));
    }
    for (key, clip) in CLIP_KEYS.iter().zip(clips) {
        match clip {
            Some(clip) => state.insert((*key).to_owned(), Value::from(clip)),
            None => state.remove(*key),
        };
    }
    if !state.is_empty() {
        custom.insert("iw4l".to_owned(), Value::Object(state));
    }
    if !custom.is_empty() {
        components.insert(CUSTOM_DATA.to_owned(), Value::Object(custom));
    }
    stack.components = (!components.is_empty()).then_some(Value::Object(components));
    stack
}

pub fn refit(stack: &ItemStack, attachments: &[String], rules: &dyn GunRules) -> Refit {
    let before = self::attachments(stack);
    let old = rules.clip_size(&stack.id, &before);
    let new = rules.clip_size(&stack.id, attachments);
    let mut rounds = 0u32;
    let mut cut = |clip: Option<i64>| {
        let loaded = match (clip?, old) {
            (FULL_CLIP, Some(size)) => i64::from(size),
            (FULL_CLIP, None) => return Some(FULL_CLIP),
            (clip, _) => clip.max(0),
        };
        match new.map(i64::from) {
            Some(size) if loaded > size => {
                rounds += u32::try_from(loaded - size).unwrap_or(0);
                Some(size)
            }
            _ => Some(loaded),
        }
    };
    let [clip, clip_left, alt_clip] = clips(stack);
    let akimbo = attachments.iter().any(|name| name == AKIMBO);
    let alt = fork_items::has_alt_mode(attachments) && fork_items::has_alt_mode(&before);
    let clips = [
        cut(clip),
        if akimbo { cut(clip_left) } else { None },
        if alt { alt_clip } else { None },
    ];
    Refit {
        stack: with_state(stack, attachments, clips),
        rounds,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    struct Sizes;

    impl GunRules for Sizes {
        fn accepts(&self, _id: &str, attachments: &[String]) -> bool {
            attachments.len() <= 2
        }

        fn clip_size(&self, _id: &str, attachments: &[String]) -> Option<i32> {
            Some(if attachments.iter().any(|name| name == "xmags") {
                45
            } else {
                30
            })
        }
    }

    fn gun(state: Value) -> ItemStack {
        let mut stack = ItemStack::new("iw4l:m4", 1);
        stack.max = 1;
        stack.components = Some(json!({
            "minecraft:custom_name": "Old Faithful",
            "minecraft:custom_data": {"iw4l": state, "other_pack": 3}
        }));
        stack
    }

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn a_smaller_clip_gives_the_extra_rounds_back_and_keeps_every_other_component() {
        let full = gun(json!({"attachments": ["xmags", "reflex"], "clip": 40, "alt_clip": 1}));
        let refit = refit(&full, &names(&["reflex"]), &Sizes);
        assert_eq!(refit.rounds, 10);
        assert_eq!(
            refit.stack.components,
            Some(json!({
                "minecraft:custom_name": "Old Faithful",
                "minecraft:custom_data": {"iw4l": {"attachments": ["reflex"], "clip": 30}, "other_pack": 3}
            }))
        );
        let unknown = gun(json!({"attachments": ["xmags"], "clip": -1}));
        let refit = super::refit(&unknown, &[], &Sizes);
        assert_eq!((clip(&refit.stack, "clip"), refit.rounds), (Some(30), 15));
        let bigger = super::refit(&gun(json!({"clip": 12})), &names(&["xmags"]), &Sizes);
        assert_eq!((clip(&bigger.stack, "clip"), bigger.rounds), (Some(12), 0));
        let empty = super::refit(&gun(json!({})), &names(&["xmags"]), &Sizes);
        assert_eq!((clip(&empty.stack, "clip"), empty.rounds), (None, 0));
    }

    #[test]
    fn a_bare_state_leaves_no_empty_custom_data() {
        let mut plain = ItemStack::new("iw4l:m4", 1);
        plain.components =
            Some(json!({"minecraft:custom_data": {"iw4l": {"attachments": ["reflex"]}}}));
        assert_eq!(with_state(&plain, &[], [None; 3]).components, None);
    }
}
