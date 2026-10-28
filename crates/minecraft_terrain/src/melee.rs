use minecraftoss_player::inventory::ItemStack;

pub const SHARPNESS: &str = "minecraft:sharpness";
pub const KNOCKBACK: &str = "minecraft:knockback";

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WeaponBonus {
    pub damage: f32,
    pub knockback: f32,
}

pub fn level(stack: Option<&ItemStack>, enchantment: &str) -> i32 {
    stack.map_or(0, |stack| {
        crate::mining_speed::enchantment_levels(stack)
            .into_iter()
            .filter(|(id, _)| id == enchantment)
            .map(|(_, level)| level)
            .max()
            .unwrap_or(0)
    })
}

pub fn sharpness_damage(level: i32) -> f32 {
    if level <= 0 {
        0.0
    } else {
        1.0 + 0.5 * (level - 1) as f32
    }
}

pub fn knockback_strength(level: i32) -> f32 {
    level.max(0) as f32 / 2.0
}

pub fn weapon_bonus(stack: Option<&ItemStack>) -> WeaponBonus {
    WeaponBonus {
        damage: sharpness_damage(level(stack, SHARPNESS)),
        knockback: knockback_strength(level(stack, KNOCKBACK)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sword(enchantments: serde_json::Value) -> ItemStack {
        let mut stack = ItemStack::new("minecraft:iron_sword", 1);
        stack.components = Some(serde_json::json!({ "minecraft:enchantments": enchantments }));
        stack
    }

    #[test]
    fn sharpness_adds_one_then_a_half_per_level_above_the_first() {
        assert_eq!(sharpness_damage(0), 0.0);
        assert_eq!(sharpness_damage(1), 1.0);
        assert_eq!(sharpness_damage(5), 3.0);
        let bonus = weapon_bonus(Some(&sword(serde_json::json!({ "sharpness": 5 }))));
        assert_eq!(
            bonus,
            WeaponBonus {
                damage: 3.0,
                knockback: 0.0
            }
        );
    }

    #[test]
    fn knockback_adds_half_a_strength_per_level() {
        assert_eq!(knockback_strength(2), 1.0);
        let stack = sword(serde_json::json!({ "minecraft:knockback": 2 }));
        assert_eq!(weapon_bonus(Some(&stack)).knockback, 1.0);
        assert_eq!(weapon_bonus(None), WeaponBonus::default());
        assert_eq!(
            weapon_bonus(Some(&ItemStack::new("minecraft:stick", 1))),
            WeaponBonus::default()
        );
    }
}
