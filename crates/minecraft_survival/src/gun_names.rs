use std::collections::HashMap;

use minecraft_terrain::fork_items::{self, Bucket, ForkItem, Kind};
use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

pub trait GunContent {
    fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32>;
    fn selection(&self, index: u32) -> Option<(String, Vec<String>)>;
    fn clip_size(&self, index: u32) -> Option<i32>;
    fn bucket(&self, _index: u32) -> Option<Bucket> {
        None
    }
}

#[derive(Debug, Default)]
pub struct GunNames {
    forward: HashMap<(&'static str, Vec<String>), Option<u32>>,
    backward: HashMap<u32, Option<(&'static str, Vec<String>)>>,
    clips: HashMap<u32, Option<i32>>,
}

fn named(id: &str) -> Option<&'static ForkItem> {
    fork_items::get(id).filter(|row| {
        row.weapon.is_some() && matches!(row.kind, Kind::Gun | Kind::Lethal | Kind::Tactical)
    })
}

fn row_for_key(key: &str) -> Option<&'static ForkItem> {
    let other = match key.strip_prefix("iw5:weapon/") {
        Some(rest) => match rest.strip_prefix("iw5_") {
            Some(bare) => format!("iw5:weapon/{bare}"),
            None => format!("iw5:weapon/iw5_{rest}"),
        },
        None => String::new(),
    };
    fork_items::by_weapon(key).or_else(|| fork_items::by_weapon(&other))
}

impl GunNames {
    pub fn weapon(
        &mut self,
        content: &impl GunContent,
        id: &str,
        attachments: &[String],
    ) -> Option<u32> {
        let row = named(id)?;
        let attachments = if row.kind == Kind::Gun {
            attachments.to_vec()
        } else {
            Vec::new()
        };
        let key = (row.id, attachments);
        if let Some(known) = self.forward.get(&key) {
            return *known;
        }
        let found = content.configuration(row.weapon?, &key.1);
        if let Some(index) = found {
            self.backward
                .entry(index)
                .or_insert_with(|| Some((row.id, key.1.clone())));
        }
        self.forward.insert(key, found);
        found
    }

    pub fn weapon_of(&mut self, content: &impl GunContent, stack: &ItemStack) -> Option<u32> {
        named(&stack.id)?;
        self.weapon(content, &stack.id, &attachments(stack))
    }

    pub fn item(
        &mut self,
        content: &impl GunContent,
        index: u32,
    ) -> Option<(&'static str, Vec<String>)> {
        if let Some(known) = self.backward.get(&index) {
            return known.clone();
        }
        let found = content.selection(index).and_then(|(key, attachments)| {
            let row = row_for_key(&key)?;
            Some((row.id, attachments))
        });
        self.backward.insert(index, found.clone());
        found
    }

    pub fn clip_size(&mut self, content: &impl GunContent, index: u32) -> Option<i32> {
        *self
            .clips
            .entry(index)
            .or_insert_with(|| content.clip_size(index))
    }
}

fn state(stack: &ItemStack) -> Option<&serde_json::Map<String, Value>> {
    stack
        .components
        .as_ref()?
        .get(fork_items::CUSTOM_DATA)?
        .get("iw4l")?
        .as_object()
}

pub fn attachments(stack: &ItemStack) -> Vec<String> {
    state(stack)
        .and_then(|state| state.get("attachments"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

pub fn clip(stack: &ItemStack, key: &str) -> Option<i64> {
    state(stack)?.get(key)?.as_i64()
}

pub fn attachment_name(name: &str, item_name: impl Fn(&str) -> String) -> String {
    match fork_items::by_alias(name) {
        Some(row) => item_name(row.id),
        None if name == "akimbo" => "Akimbo".to_owned(),
        None => name.to_owned(),
    }
}

fn rounds(clip: Option<i64>, size: Option<i32>) -> String {
    let loaded = match (clip, size) {
        (Some(fork_items::FULL_CLIP), Some(size)) => i64::from(size),
        (Some(fork_items::FULL_CLIP), None) => return "full".to_owned(),
        (clip, _) => clip.unwrap_or(0).max(0),
    };
    match size {
        Some(size) => format!("{loaded}/{size}"),
        None => loaded.to_string(),
    }
}

pub fn tooltip_lines(
    attachments: &[String],
    clips: (Option<i64>, Option<i64>),
    size: Option<i32>,
    ammo: Option<&str>,
) -> Vec<String> {
    let mut lines = attachments.to_vec();
    let Some(ammo) = ammo else {
        return lines;
    };
    let (clip, left) = clips;
    let mut line = format!("Clip {}", rounds(clip, size));
    if let Some(left) = left {
        line.push_str(&format!(" + {}", rounds(Some(left), size)));
    }
    lines.push(line);
    lines.push(format!("Ammo: {ammo}"));
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        weapons: Vec<(&'static str, Vec<&'static str>)>,
    }

    impl Fake {
        fn new() -> Self {
            Self {
                weapons: vec![
                    ("", Vec::new()),
                    ("iw4:weapon/m4_mp", Vec::new()),
                    ("iw4:weapon/m4_mp", vec!["reflex"]),
                    ("iw4:weapon/m4_mp", vec!["reflex", "fmj"]),
                    ("iw4:weapon/frag_grenade_mp", Vec::new()),
                    ("iw5:weapon/iw5_xm25_mp", Vec::new()),
                    ("iw4:weapon/killstreak_ac130_mp", Vec::new()),
                ],
            }
        }
    }

    impl GunContent for Fake {
        fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32> {
            let weapon = weapon.replace("iw5:weapon/xm25_mp", "iw5:weapon/iw5_xm25_mp");
            let index = self.weapons.iter().position(|(key, list)| {
                *key == weapon
                    && list
                        .iter()
                        .map(|name| name.to_string())
                        .eq(attachments.iter().cloned())
            })?;
            u32::try_from(index).ok()
        }

        fn selection(&self, index: u32) -> Option<(String, Vec<String>)> {
            let (key, list) = self
                .weapons
                .get(index as usize)
                .filter(|(key, _)| !key.is_empty())?;
            Some((
                key.to_string(),
                list.iter().map(|name| name.to_string()).collect(),
            ))
        }

        fn clip_size(&self, index: u32) -> Option<i32> {
            (index > 0).then_some(30)
        }
    }

    fn gun(id: &str, attachments: &[&str]) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.components = Some(serde_json::json!({
            "minecraft:custom_data": {"iw4l": {"attachments": attachments, "clip": 27}}
        }));
        stack
    }

    #[test]
    fn gun_names_go_both_ways_and_cache() {
        let fake = Fake::new();
        let mut names = GunNames::default();
        assert_eq!(names.weapon(&fake, "iw4l:m4", &[]), Some(1));
        assert_eq!(
            names.weapon_of(&fake, &gun("iw4l:m4", &["reflex"])),
            Some(2)
        );
        assert_eq!(
            names.weapon_of(&fake, &gun("iw4l:m4", &["reflex", "fmj"])),
            Some(3)
        );
        assert_eq!(
            names.weapon_of(&fake, &gun("iw4l:m4", &["acog", "silencer"])),
            None
        );
        assert_eq!(
            names.weapon(&fake, "iw4l:frag_grenade", &["ignored".to_owned()]),
            Some(4)
        );
        assert_eq!(names.weapon(&fake, "iw4l:iw5_xm25", &[]), Some(5));
        assert_eq!(names.weapon(&fake, "iw4l:rifle_ammo", &[]), None);
        assert_eq!(names.weapon(&fake, "minecraft:stone", &[]), None);
        assert_eq!(
            names.weapon_of(&fake, &ItemStack::new("iw4:weapon/3", 1)),
            None
        );
        for index in 1..=5 {
            let (id, attachments) = names.item(&fake, index).unwrap();
            assert_eq!(names.weapon(&fake, id, &attachments), Some(index), "{id}");
        }
        assert_eq!(
            names.item(&fake, 2),
            Some(("iw4l:m4", vec!["reflex".to_owned()]))
        );
        let mut fresh = GunNames::default();
        assert_eq!(fresh.item(&fake, 5), Some(("iw4l:iw5_xm25", Vec::new())));
        assert_eq!(fresh.item(&fake, 6), None);
        assert_eq!(fresh.item(&fake, 0), None);
        assert_eq!(fresh.clip_size(&fake, 2), Some(30));
    }

    #[test]
    fn the_item_state_is_read_from_custom_data() {
        let stack = gun("iw4l:m4", &["reflex", "fmj"]);
        assert_eq!(attachments(&stack), ["reflex", "fmj"]);
        assert_eq!(clip(&stack, "clip"), Some(27));
        assert_eq!(clip(&stack, "clip_left"), None);
        assert!(attachments(&ItemStack::new("iw4l:m4", 1)).is_empty());
    }

    #[test]
    fn tooltip_lines_list_attachments_clip_and_ammo() {
        let name =
            |id: &str| fork_items::get(id).map_or_else(String::new, |row| row.name.to_owned());
        let shown: Vec<String> = ["reflex", "akimbo", "flamethrower"]
            .iter()
            .map(|alias| attachment_name(alias, name))
            .collect();
        assert_eq!(shown, ["Red Dot Sight", "Akimbo", "flamethrower"]);
        assert_eq!(
            tooltip_lines(&shown[..1], (Some(27), None), Some(30), Some("Rifle Ammo")),
            ["Red Dot Sight", "Clip 27/30", "Ammo: Rifle Ammo"]
        );
        assert_eq!(
            tooltip_lines(&[], (None, None), Some(30), Some("Rifle Ammo")),
            ["Clip 0/30", "Ammo: Rifle Ammo"]
        );
        assert_eq!(
            tooltip_lines(&[], (Some(-1), Some(12)), Some(15), Some("Pistol Ammo")),
            ["Clip 15/15 + 12/15", "Ammo: Pistol Ammo"]
        );
        assert_eq!(
            tooltip_lines(&[], (Some(-1), None), None, Some("Rockets")),
            ["Clip full", "Ammo: Rockets"]
        );
        assert_eq!(
            tooltip_lines(&[], (None, None), None, None),
            Vec::<String>::new()
        );
    }
}
