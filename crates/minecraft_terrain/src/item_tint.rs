use minecraftoss_player::inventory::ItemStack;

use crate::potions;

pub const KEY_MARK: char = '~';

pub fn key(id: &str, potion: Option<u32>) -> String {
    match potion {
        Some(rgb) => format!("{id}{KEY_MARK}{:06x}", rgb & 0xFFFFFF),
        None => id.to_owned(),
    }
}

pub fn split(key: &str) -> (&str, Option<u32>) {
    match key.rsplit_once(KEY_MARK) {
        Some((id, rgb)) if rgb.len() == 6 && rgb.bytes().all(|b| b.is_ascii_hexdigit()) => {
            (id, u32::from_str_radix(rgb, 16).ok())
        }
        Some((id, _)) => (id, None),
        None => (key, None),
    }
}

pub fn of_stack(stack: &ItemStack) -> String {
    key(&stack.id, potions::stack_color(stack.components.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_key_splits_back_into_its_id_and_colour() {
        assert_eq!(key("minecraft:potion", None), "minecraft:potion");
        assert_eq!(
            key("minecraft:potion", Some(0xF82423)),
            "minecraft:potion~f82423"
        );
        assert_eq!(
            split("minecraft:potion~f82423"),
            ("minecraft:potion", Some(0xF82423))
        );
        assert_eq!(split("minecraft:potion"), ("minecraft:potion", None));
        assert_eq!(
            split(&key("minecraft:tipped_arrow", Some(0x0000ff))),
            ("minecraft:tipped_arrow", Some(0x0000ff))
        );
    }

    #[test]
    fn a_dial_key_is_left_for_the_dial_split() {
        let dial = crate::dials::keyed("minecraft:clock", "minecraft:item/clock_03");
        assert_eq!(split(&dial), (dial.as_str(), None));
        let tinted = key(&dial, Some(0x123456));
        let (rest, potion) = split(&tinted);
        assert_eq!((rest, potion), (dial.as_str(), Some(0x123456)));
        assert_eq!(
            crate::dials::split(rest),
            ("minecraft:clock", Some("minecraft:item/clock_03"))
        );
    }

    #[test]
    fn a_bad_suffix_has_no_colour() {
        assert_eq!(split("minecraft:potion~zz0000"), ("minecraft:potion", None));
        assert_eq!(split("minecraft:potion~f8242"), ("minecraft:potion", None));
        assert_eq!(split("minecraft:potion~"), ("minecraft:potion", None));
    }

    #[test]
    fn a_poison_splash_potion_keys_its_colour() {
        let mut stack = ItemStack::new("minecraft:splash_potion", 1);
        stack.components =
            Some(json!({"minecraft:potion_contents": {"potion": "minecraft:poison"}}));
        assert_eq!(of_stack(&stack), "minecraft:splash_potion~87a363");
        assert_eq!(
            of_stack(&ItemStack::new("minecraft:potion", 1)),
            "minecraft:potion"
        );
        assert_eq!(
            of_stack(&ItemStack::new("minecraft:stone", 64)),
            "minecraft:stone"
        );
    }
}
