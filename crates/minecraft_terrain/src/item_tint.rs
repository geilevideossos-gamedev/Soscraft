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
    of_stack_at(&crate::dials::stack_key(stack), stack)
}

pub fn of_stack_at(base: &str, stack: &ItemStack) -> String {
    let color = potions::stack_color(stack.components.as_ref())
        .or_else(|| crate::body_gear::dyed_color(stack));
    key(base, color)
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
    fn a_lodestone_compass_keys_its_target_and_still_splits_to_the_compass() {
        let compass = ItemStack::new("minecraft:compass", 1);
        assert_eq!(of_stack(&compass), "minecraft:compass");
        let target = crate::lodestone::Target::new(crate::terrain::Dimension::End, (-4, 70, 12));
        let tracked = of_stack(&crate::lodestone::with_tracker(&compass, target));
        assert_eq!(
            tracked,
            "minecraft:compass@lodestone:minecraft:the_end:-4,70,12"
        );
        assert_eq!(split(&tracked), (tracked.as_str(), None));
        assert_eq!(crate::dials::split(&tracked), ("minecraft:compass", None));
        let dial = crate::dials::keyed("minecraft:compass", "minecraft:item/compass_07");
        assert_eq!(
            crate::dials::split(&dial),
            ("minecraft:compass", Some("minecraft:item/compass_07"))
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

    #[test]
    fn a_dyed_stack_keys_its_dye_and_a_dial_base_keeps_its_model() {
        let mut armour = ItemStack::new("minecraft:leather_horse_armor", 1);
        armour.components = Some(json!({"minecraft:dyed_color": 0xB02E26}));
        assert_eq!(of_stack(&armour), "minecraft:leather_horse_armor~b02e26");
        armour.components = Some(json!({"minecraft:dyed_color": {"rgb": 0x3C44AA}}));
        assert_eq!(
            split(&of_stack(&armour)),
            ("minecraft:leather_horse_armor", Some(0x3C44AA))
        );
        assert_eq!(
            of_stack(&ItemStack::new("minecraft:leather_horse_armor", 1)),
            "minecraft:leather_horse_armor"
        );
        let compass = ItemStack::new("minecraft:compass", 1);
        let dial = crate::dials::keyed("minecraft:compass", "minecraft:item/compass_16");
        let held = of_stack_at(&dial, &compass);
        assert_eq!(held, dial);
        let (rest, color) = split(&held);
        assert_eq!(color, None);
        assert_eq!(
            crate::dials::split(rest),
            ("minecraft:compass", Some("minecraft:item/compass_16"))
        );
        let mut potion = ItemStack::new("minecraft:potion", 1);
        potion.components = Some(json!({
            "minecraft:potion_contents": {"potion": "minecraft:poison"},
            "minecraft:dyed_color": 0xB02E26
        }));
        assert_eq!(of_stack(&potion), "minecraft:potion~87a363");
    }
}
