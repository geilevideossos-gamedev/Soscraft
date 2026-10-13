use minecraftoss_player::inventory::ItemStack;
use serde_json::Value;

pub const OFFHAND: usize = 40;
pub const ARMOR_SLOTS: [usize; 4] = [39, 38, 37, 36];
const DYED_COLOR: &str = "minecraft:dyed_color";
const CARVED_PUMPKIN: &str = "minecraft:carved_pumpkin";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GearItem {
    pub id: String,
    pub glint: bool,
    pub dye: Option<u32>,
}

impl GearItem {
    pub fn of(stack: &ItemStack) -> Self {
        Self {
            id: stack.id.clone(),
            glint: crate::glint::has_glint(stack),
            dye: dyed_color(stack),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BodyGear {
    pub main: Option<GearItem>,
    pub off: Option<GearItem>,
    pub armor: [Option<GearItem>; 4],
    pub hides_gun: bool,
}

impl BodyGear {
    pub fn is_bare(&self) -> bool {
        self.main.is_none() && self.off.is_none() && self.armor.iter().all(Option::is_none)
    }
}

pub fn body_gear(
    slots: &[Option<ItemStack>],
    selected: usize,
    is_gun: impl Fn(&ItemStack) -> bool,
) -> BodyGear {
    let stack = |slot: usize| slots.get(slot).and_then(Option::as_ref);
    let selected_stack = stack(selected);
    let hides_gun = selected_stack.is_none_or(|stack| !is_gun(stack));
    let worn = |slot: usize| stack(slot).filter(|stack| !is_gun(stack)).map(GearItem::of);
    BodyGear {
        main: selected_stack
            .filter(|stack| !is_gun(stack))
            .map(GearItem::of),
        off: worn(OFFHAND),
        armor: ARMOR_SLOTS.map(worn),
        hides_gun,
    }
}

pub fn camera_overlay(slots: &[Option<ItemStack>]) -> bool {
    slots
        .get(ARMOR_SLOTS[0])
        .and_then(Option::as_ref)
        .is_some_and(|stack| stack.id == CARVED_PUMPKIN)
}

pub fn dyed_color(stack: &ItemStack) -> Option<u32> {
    let value = stack.components.as_ref()?.get(DYED_COLOR)?;
    let rgb = match value {
        Value::Object(fields) => fields.get("rgb")?,
        number => number,
    };
    rgb.as_i64().map(|rgb| (rgb as u32) & 0xFF_FFFF)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const GUN: &str = "iw4:weapon/7";

    fn stack(id: &str, components: Option<Value>) -> Option<ItemStack> {
        Some(ItemStack {
            components,
            ..ItemStack::new(id, 1)
        })
    }

    fn slots(items: &[(usize, Option<ItemStack>)]) -> Vec<Option<ItemStack>> {
        let mut slots = vec![None; 43];
        for (slot, item) in items {
            slots[*slot] = item.clone();
        }
        slots
    }

    fn is_gun(stack: &ItemStack) -> bool {
        stack.id.starts_with("iw4:weapon/")
    }

    fn plain(id: &str) -> Option<GearItem> {
        Some(GearItem {
            id: id.to_owned(),
            glint: false,
            dye: None,
        })
    }

    #[test]
    fn armour_is_read_head_to_feet_and_the_hands_from_the_selection_and_offhand() {
        let inventory = slots(&[
            (2, stack("minecraft:cobblestone", None)),
            (39, stack("minecraft:diamond_helmet", None)),
            (38, stack("minecraft:iron_chestplate", None)),
            (37, stack("minecraft:leather_leggings", None)),
            (36, stack("minecraft:iron_boots", None)),
            (OFFHAND, stack("minecraft:torch", None)),
        ]);
        let gear = body_gear(&inventory, 2, is_gun);
        assert_eq!(gear.main, plain("minecraft:cobblestone"));
        assert_eq!(gear.off, plain("minecraft:torch"));
        assert_eq!(
            gear.armor,
            [
                plain("minecraft:diamond_helmet"),
                plain("minecraft:iron_chestplate"),
                plain("minecraft:leather_leggings"),
                plain("minecraft:iron_boots"),
            ]
        );
        assert!(gear.hides_gun);
        assert!(!gear.is_bare());
    }

    #[test]
    fn a_selected_gun_is_left_to_the_mw2_body() {
        let inventory = slots(&[
            (0, stack(GUN, None)),
            (OFFHAND, stack("minecraft:torch", None)),
        ]);
        let gear = body_gear(&inventory, 0, is_gun);
        assert_eq!(gear.main, None);
        assert!(!gear.hides_gun);
        assert_eq!(gear.off, plain("minecraft:torch"));
    }

    #[test]
    fn an_empty_hand_puts_the_gun_away_too() {
        let gear = body_gear(&slots(&[(0, stack(GUN, None))]), 4, is_gun);
        assert_eq!(gear.main, None);
        assert!(gear.hides_gun);
        assert!(gear.is_bare());
        assert!(body_gear(&[], 0, is_gun).hides_gun);
    }

    #[test]
    fn glint_and_dye_come_from_the_stack() {
        let inventory = slots(&[
            (
                1,
                stack(
                    "minecraft:diamond_sword",
                    Some(json!({"minecraft:enchantments": {"minecraft:sharpness": 5}})),
                ),
            ),
            (
                38,
                stack(
                    "minecraft:leather_chestplate",
                    Some(json!({"minecraft:dyed_color": 11546150})),
                ),
            ),
            (
                37,
                stack(
                    "minecraft:leather_leggings",
                    Some(json!({"minecraft:dyed_color": {"rgb": 255}})),
                ),
            ),
            (36, stack("minecraft:leather_boots", None)),
        ]);
        let gear = body_gear(&inventory, 1, is_gun);
        assert_eq!(
            gear.main,
            Some(GearItem {
                id: "minecraft:diamond_sword".to_owned(),
                glint: true,
                dye: None,
            })
        );
        assert_eq!(gear.armor[1].as_ref().unwrap().dye, Some(0xB02E26));
        assert_eq!(gear.armor[2].as_ref().unwrap().dye, Some(0x0000FF));
        assert_eq!(gear.armor[3].as_ref().unwrap().dye, None);
        assert!(!gear.armor[1].as_ref().unwrap().glint);
    }

    #[test]
    fn only_a_carved_pumpkin_on_the_head_blurs_the_view() {
        let pumpkin = || stack("minecraft:carved_pumpkin", None);
        assert!(camera_overlay(&slots(&[(39, pumpkin())])));
        for slot in [36, 37, 38, 40, 0] {
            assert!(!camera_overlay(&slots(&[(slot, pumpkin())])), "{slot}");
        }
        for id in [
            "minecraft:pumpkin",
            "minecraft:jack_o_lantern",
            "minecraft:dragon_head",
            "minecraft:diamond_helmet",
        ] {
            assert!(!camera_overlay(&slots(&[(39, stack(id, None))])), "{id}");
        }
        assert!(!camera_overlay(&slots(&[])));
        assert!(!camera_overlay(&[]));
    }
}
