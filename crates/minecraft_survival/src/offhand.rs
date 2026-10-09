use minecraftoss_player::GameMode;
use minecraftoss_player::food::FoodUse;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::survival::FoodData;

pub const OFFHAND: usize = 40;
pub const SHIELD: &str = "minecraft:shield";
pub const TOTEM: &str = "minecraft:totem_of_undying";
pub const BLOCK_SOUND: &str = "minecraft:item.shield.block";
pub const BREAK_SOUND: &str = "minecraft:item.shield.break";
pub const TOTEM_SOUND: &str = "minecraft:item.totem.use";
const BLOCK_DELAY_TICKS: u32 = 5;
pub const USE_DELAY_TICKS: u32 = 4;
const BLOCKING_ANGLE_DEGREES: f32 = 90.0;
const DEGREES_TO_RADIANS: f32 = 0.017_453_292;
const WEAR_THRESHOLD: f32 = 3.0;
const WEAR_BASE: f32 = 1.0;
const WEAR_FACTOR: f32 = 1.0;
const REDUCTION_BASE: f32 = 0.0;
const REDUCTION_FACTOR: f32 = 1.0;
const USED_ITEMS: [&str; 10] = [
    "minecraft:bucket",
    "minecraft:water_bucket",
    "minecraft:lava_bucket",
    "minecraft:powder_snow_bucket",
    "minecraft:flint_and_steel",
    "minecraft:bone_meal",
    "minecraft:glass_bottle",
    "minecraft:potion",
    minecraft_terrain::bottle::MILK_BUCKET,
    SHIELD,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandUse {
    Consumed,
    Failed,
    Passed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocking {
    pub slot: usize,
    pub ticks: u32,
}

impl Blocking {
    pub fn start(slot: usize) -> Self {
        Self { slot, ticks: 0 }
    }

    pub fn raised(&self) -> bool {
        self.ticks >= BLOCK_DELAY_TICKS
    }

    pub fn holds_shield(&self, inventory: &Inventory, selected: usize) -> bool {
        (self.slot == OFFHAND || self.slot == selected)
            && inventory
                .slots
                .get(self.slot)
                .and_then(Option::as_ref)
                .is_some_and(|stack| stack.id == SHIELD)
    }
}

pub fn hand_slots(selected: usize, main_hand_is_gun: bool) -> Vec<usize> {
    if main_hand_is_gun {
        vec![OFFHAND]
    } else {
        vec![selected, OFFHAND]
    }
}

pub fn use_hands(
    hands: &[usize],
    mut use_hand: impl FnMut(usize) -> HandUse,
) -> Option<(usize, HandUse)> {
    hands
        .iter()
        .map(|&slot| (slot, use_hand(slot)))
        .find(|&(_, used)| used != HandUse::Passed)
}

pub fn repeat_delay(used: Option<(usize, HandUse)>, delay: u32) -> u32 {
    if matches!(used, Some((_, HandUse::Consumed))) {
        USE_DELAY_TICKS
    } else {
        delay
    }
}

pub fn has_use(stack: &ItemStack, placeable: bool, food: &FoodData, mode: GameMode) -> bool {
    placeable
        || USED_ITEMS.contains(&stack.id.as_str())
        || stack.id == minecraft_terrain::berries::SWEET_BERRIES
        || stack.id == minecraft_terrain::berries::GLOW_BERRIES
        || FoodUse::start(OFFHAND, stack, food, mode).is_some()
        || minecraft_terrain::thrown::Throwable::of(&stack.id).is_some()
        || stack.id == minecraft_terrain::trident::ITEM
        || stack.id == minecraft_terrain::ender_eye::ENDER_EYE
        || stack.id == minecraft_terrain::firework::FIREWORK_ROCKET
        || minecraft_terrain::spawn_egg::spawned_kind(&stack.id).is_some()
}

pub fn swap_hands(inventory: &mut Inventory, selected: usize) -> bool {
    let gun = |slot: usize| {
        inventory.slots[slot]
            .as_ref()
            .is_some_and(|stack| crate::guns::weapon_of(stack).is_some())
    };
    if selected >= crate::guns::HOTBAR || gun(selected) || gun(OFFHAND) {
        return false;
    }
    inventory.slots.swap(selected, OFFHAND);
    true
}

pub fn blocked_damage(
    yaw_degrees: f32,
    toward_source: Option<[f64; 2]>,
    damage: f32,
) -> f32 {
    let angle = toward_source.map_or(std::f64::consts::PI, |[x, z]| {
        let length = x.hypot(z);
        let (x, z) = if length < 1.0e-5 {
            (0.0, 0.0)
        } else {
            (x / length, z / length)
        };
        let yaw = f64::from(-yaw_degrees * DEGREES_TO_RADIANS);
        (yaw.sin() * x + yaw.cos() * z).acos()
    });
    if angle > f64::from(DEGREES_TO_RADIANS * BLOCKING_ANGLE_DEGREES) {
        return 0.0;
    }
    let reduced = REDUCTION_BASE + REDUCTION_FACTOR * damage;
    if reduced < 0.0 {
        0.0
    } else {
        reduced.min(damage)
    }
}

pub fn shield_wear(blocked: f32) -> u32 {
    if blocked < WEAR_THRESHOLD {
        0
    } else {
        (WEAR_BASE + WEAR_FACTOR * blocked).floor() as u32
    }
}

pub fn totem_slot(inventory: &Inventory, selected: usize) -> Option<usize> {
    [selected, OFFHAND].into_iter().find(|&slot| {
        inventory
            .slots
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|stack| stack.id == TOTEM && stack.count > 0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inventory(stacks: &[(usize, &str)]) -> Inventory {
        let mut inventory = Inventory::default();
        for &(slot, id) in stacks {
            inventory.slots[slot] = Some(ItemStack::new(id, 1));
        }
        inventory
    }

    #[test]
    fn the_main_hand_goes_first_and_the_offhand_only_when_it_passes() {
        let mut tried = Vec::new();
        let used = use_hands(&hand_slots(2, false), |slot| {
            tried.push(slot);
            if slot == OFFHAND {
                HandUse::Consumed
            } else {
                HandUse::Passed
            }
        });
        assert_eq!(used, Some((OFFHAND, HandUse::Consumed)));
        assert_eq!(tried, vec![2, OFFHAND]);
        tried.clear();
        let used = use_hands(&hand_slots(2, false), |slot| {
            tried.push(slot);
            HandUse::Consumed
        });
        assert_eq!(used, Some((2, HandUse::Consumed)));
        assert_eq!(tried, vec![2]);
        tried.clear();
        let used = use_hands(&hand_slots(2, false), |slot| {
            tried.push(slot);
            if slot == 2 {
                HandUse::Failed
            } else {
                HandUse::Consumed
            }
        });
        assert_eq!(used, Some((2, HandUse::Failed)));
        assert_eq!(tried, vec![2]);
        assert_eq!(use_hands(&hand_slots(2, false), |_| HandUse::Passed), None);
    }

    #[test]
    fn a_use_by_either_hand_waits_four_ticks_even_after_a_failed_main_hand_placement() {
        let mut delay = USE_DELAY_TICKS;
        let used = use_hands(&hand_slots(2, false), |slot| {
            if slot == OFFHAND {
                HandUse::Consumed
            } else {
                delay = 0;
                HandUse::Passed
            }
        });
        assert_eq!(repeat_delay(used, delay), USE_DELAY_TICKS);
        let mut delay = USE_DELAY_TICKS;
        let used = use_hands(&hand_slots(2, false), |_| {
            delay = 0;
            HandUse::Passed
        });
        assert_eq!(repeat_delay(used, delay), 0);
        assert_eq!(repeat_delay(Some((2, HandUse::Failed)), 0), 0);
        assert_eq!(repeat_delay(None, 3), 3);
    }

    #[test]
    fn a_gun_in_the_main_hand_leaves_only_the_offhand() {
        let mut tried = Vec::new();
        use_hands(&hand_slots(0, true), |slot| {
            tried.push(slot);
            HandUse::Passed
        });
        assert_eq!(tried, vec![OFFHAND]);
    }

    #[test]
    fn only_items_with_a_use_take_the_tactical_key() {
        let has_use = |stack: &ItemStack, placeable: bool, food: &FoodData| {
            super::has_use(stack, placeable, food, GameMode::Survival)
        };
        let stack = |id: &str| ItemStack::new(id, 1);
        let full = FoodData::default();
        let mut hungry = FoodData::default();
        hungry.level = 12;
        assert!(has_use(&stack("minecraft:torch"), true, &full));
        assert!(has_use(&stack(SHIELD), false, &full));
        assert!(has_use(&stack("minecraft:bread"), false, &hungry));
        assert!(!has_use(&stack("minecraft:bread"), false, &full));
        assert!(has_use(&stack("minecraft:golden_apple"), false, &full));
        assert!(has_use(
            &stack(minecraft_terrain::berries::SWEET_BERRIES),
            false,
            &full
        ));
        assert!(has_use(&stack("minecraft:snowball"), false, &full));
        assert!(has_use(&stack("minecraft:trident"), false, &full));
        assert!(has_use(&stack("minecraft:ender_eye"), false, &full));
        assert!(has_use(&stack("minecraft:firework_rocket"), false, &full));
        assert!(has_use(&stack("minecraft:water_bucket"), false, &full));
        assert!(has_use(&stack("minecraft:milk_bucket"), false, &full));
        assert!(has_use(&stack("minecraft:zombie_spawn_egg"), false, &full));
        assert!(!has_use(&stack(TOTEM), false, &hungry));
        assert!(!has_use(&stack("minecraft:stick"), false, &hungry));
    }

    #[test]
    fn creative_players_can_eat_from_the_offhand_at_full_hunger() {
        let bread = ItemStack::new("minecraft:bread", 1);
        let full = FoodData::default();
        assert!(has_use(&bread, false, &full, GameMode::Creative));
        assert!(!has_use(&bread, false, &full, GameMode::Adventure));
        assert!(!has_use(&bread, false, &full, GameMode::Survival));
    }

    #[test]
    fn swapping_hands_trades_the_selected_slot_with_the_offhand_but_never_a_gun() {
        let mut inv = inventory(&[(3, "minecraft:torch"), (OFFHAND, SHIELD)]);
        assert!(swap_hands(&mut inv, 3));
        assert_eq!(inv.slots[3].as_ref().map(|s| s.id.as_str()), Some(SHIELD));
        assert_eq!(
            inv.slots[OFFHAND].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:torch")
        );
        assert!(swap_hands(&mut inv, 5));
        assert!(inv.slots[OFFHAND].is_none());
        assert_eq!(
            inv.slots[5].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:torch")
        );
        let mut armed = inventory(&[(0, "iw4:weapon/7"), (OFFHAND, SHIELD)]);
        assert!(!swap_hands(&mut armed, 0));
        assert!(armed.slots[0].is_some() && armed.slots[OFFHAND].is_some());
    }

    #[test]
    fn the_shield_blocks_after_five_ticks_while_its_hand_holds_it() {
        let inv = inventory(&[(OFFHAND, SHIELD), (1, SHIELD)]);
        let mut blocking = Blocking::start(OFFHAND);
        blocking.ticks = 4;
        assert!(!blocking.raised());
        blocking.ticks = 5;
        assert!(blocking.raised());
        assert!(blocking.holds_shield(&inv, 0));
        assert!(Blocking::start(1).holds_shield(&inv, 1));
        assert!(!Blocking::start(1).holds_shield(&inv, 2));
        assert!(!Blocking::start(OFFHAND).holds_shield(&inventory(&[]), 0));
    }

    #[test]
    fn the_shield_takes_all_damage_from_the_front_ninety_degrees() {
        assert_eq!(blocked_damage(0.0, Some([0.0, 5.0]), 6.0), 6.0);
        assert_eq!(blocked_damage(0.0, Some([3.0, 3.1]), 6.0), 6.0);
        assert_eq!(blocked_damage(0.0, Some([3.0, -0.1]), 6.0), 0.0);
        assert_eq!(blocked_damage(0.0, Some([0.0, -5.0]), 6.0), 0.0);
        assert_eq!(blocked_damage(90.0, Some([-5.0, 0.0]), 4.0), 4.0);
        assert_eq!(blocked_damage(90.0, Some([5.0, 0.0]), 4.0), 0.0);
        assert_eq!(blocked_damage(180.0, Some([0.0, -2.0]), 4.0), 4.0);
        assert_eq!(blocked_damage(0.0, None, 4.0), 0.0);
    }

    #[test]
    fn the_shield_wears_one_plus_the_damage_from_three_damage() {
        assert_eq!(shield_wear(0.0), 0);
        assert_eq!(shield_wear(2.9), 0);
        assert_eq!(shield_wear(3.0), 4);
        assert_eq!(shield_wear(4.5), 5);
        assert_eq!(shield_wear(10.0), 11);
    }

    #[test]
    fn a_totem_in_the_main_hand_is_spent_before_the_offhand() {
        assert_eq!(
            totem_slot(&inventory(&[(4, TOTEM), (OFFHAND, TOTEM)]), 4),
            Some(4)
        );
        assert_eq!(
            totem_slot(&inventory(&[(4, TOTEM), (OFFHAND, TOTEM)]), 3),
            Some(OFFHAND)
        );
        assert_eq!(totem_slot(&inventory(&[(5, TOTEM)]), 4), None);
    }
}
