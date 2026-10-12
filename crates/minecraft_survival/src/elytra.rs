use minecraft_terrain::trident::{enchantment, next_damage_breaks};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::survival::{EffectKind, SurvivalStatus};
use serde_json::Value;

pub const ELYTRA: &str = "minecraft:elytra";
pub const CHEST: usize = 38;
const GLIDER: &str = "minecraft:glider";
const REMOVED_GLIDER: &str = "!minecraft:glider";
const TICK_SECONDS: f64 = 0.05;
const WEAR_TICKS: u32 = 20;
const SLOW_FALL: f64 = -0.5;
const WALL_SCALE: f64 = 10.0;
const WALL_GRACE: f64 = 3.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Flight {
    pub gliding: bool,
    pub motion: [f64; 3],
    pub against_wall: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Glided {
    pub wall_damage: Option<f32>,
    pub worn: u32,
}

#[derive(Default)]
pub struct Elytra {
    clock: f64,
    fall_fly_ticks: u32,
    last_speed: Option<f64>,
    random: LegacyRandom,
}

pub fn is_glider(stack: &ItemStack) -> bool {
    let patch = stack.components.as_ref().and_then(Value::as_object);
    if patch.is_some_and(|patch| patch.contains_key(GLIDER)) {
        return true;
    }
    stack.id == ELYTRA && !patch.is_some_and(|patch| patch.contains_key(REMOVED_GLIDER))
}

pub fn can_glide(inventory: &Inventory) -> bool {
    inventory
        .slots
        .get(CHEST)
        .and_then(Option::as_ref)
        .is_some_and(|stack| is_glider(stack) && !next_damage_breaks(inventory, stack))
}

pub fn wall_damage(old_speed: f64, new_speed: f64) -> Option<f32> {
    let damage = ((old_speed - new_speed) * WALL_SCALE - WALL_GRACE) as f32;
    (damage > 0.0).then_some(damage)
}

pub fn fall_peak(peak: Option<f64>, feet_y: f64, flight: Flight) -> Option<f64> {
    if flight.gliding && flight.motion[1] > SLOW_FALL {
        peak.map(|peak| peak.min(feet_y + 1.0))
    } else {
        peak
    }
}

pub fn floating(status: &SurvivalStatus) -> bool {
    status.effect(EffectKind::Levitation).is_some()
        || status.effect(EffectKind::SlowFalling).is_some()
}

pub fn float_peak(peak: Option<f64>, feet_y: f64, floating: bool) -> Option<f64> {
    if floating {
        peak.map(|_| feet_y)
    } else {
        peak
    }
}

impl Elytra {
    pub fn step(
        &mut self,
        flight: Flight,
        seconds: f64,
        inventory: &mut Inventory,
        wears: bool,
    ) -> Glided {
        let mut glided = Glided::default();
        let speed = flight.motion[0].hypot(flight.motion[2]);
        if flight.gliding {
            glided.wall_damage = self
                .last_speed
                .filter(|_| flight.against_wall)
                .and_then(|old| wall_damage(old, speed));
            self.last_speed = Some(speed);
        } else {
            self.last_speed = None;
        }
        self.clock += seconds.max(0.0);
        let ticks = (self.clock / TICK_SECONDS) as u32;
        self.clock -= f64::from(ticks) * TICK_SECONDS;
        for _ in 0..ticks {
            if !flight.gliding {
                self.fall_fly_ticks = 0;
                continue;
            }
            self.fall_fly_ticks += 1;
            if self.fall_fly_ticks.is_multiple_of(WEAR_TICKS)
                && wears
                && wear(inventory, &mut self.random)
            {
                glided.worn += 1;
            }
        }
        glided
    }
}

fn wear(inventory: &mut Inventory, random: &mut LegacyRandom) -> bool {
    let Some(stack) = inventory.slots.get(CHEST).cloned().flatten() else {
        return false;
    };
    let unbreaking = enchantment(&stack, "minecraft:unbreaking");
    if unbreaking > 0 && random.next_float() < unbreaking as f32 / (unbreaking + 1) as f32 {
        return false;
    }
    inventory.wear_tool(CHEST, 1);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn wearing(components: Option<Value>) -> Inventory {
        let mut inventory = Inventory::default();
        let mut elytra = ItemStack::new(ELYTRA, 1);
        elytra.components = components;
        inventory.slots[CHEST] = Some(elytra);
        inventory
    }

    fn damage(inventory: &Inventory) -> u64 {
        inventory.slots[CHEST]
            .as_ref()
            .and_then(|stack| stack.components.as_ref())
            .and_then(|components| components.get("minecraft:damage"))
            .and_then(Value::as_u64)
            .unwrap_or(0)
    }

    const GLIDING: Flight = Flight {
        gliding: true,
        motion: [1.0, -0.2, 0.0],
        against_wall: false,
    };

    #[test]
    fn only_a_worn_glider_short_of_its_last_point_lets_a_player_glide() {
        assert!(can_glide(&wearing(None)));
        assert!(can_glide(&wearing(Some(
            json!({"minecraft:damage": 430, "minecraft:max_damage": 432})
        ))));
        assert!(!can_glide(&wearing(Some(
            json!({"minecraft:damage": 431, "minecraft:max_damage": 432})
        ))));
        assert!(!can_glide(&wearing(Some(json!({"!minecraft:glider": {}})))));
        let mut chestplate = Inventory::default();
        chestplate.slots[CHEST] = Some(ItemStack::new("minecraft:diamond_chestplate", 1));
        assert!(!can_glide(&chestplate));
        let mut held = Inventory::default();
        held.slots[0] = Some(ItemStack::new(ELYTRA, 1));
        assert!(!can_glide(&held));
        let mut wings = ItemStack::new("minecraft:leather_chestplate", 1);
        wings.components = Some(json!({"minecraft:glider": {}}));
        assert!(is_glider(&wings));
        assert!(!can_glide(&Inventory::default()));
    }

    #[test]
    fn gliding_wears_one_point_every_twenty_ticks_but_not_in_creative() {
        let mut inventory = wearing(Some(json!({"minecraft:max_damage": 432})));
        let mut elytra = Elytra::default();
        let mut worn = 0;
        for _ in 0..19 {
            worn += elytra.step(GLIDING, 0.05, &mut inventory, true).worn;
        }
        assert_eq!((worn, damage(&inventory)), (0, 0));
        worn += elytra.step(GLIDING, 0.05, &mut inventory, true).worn;
        assert_eq!((worn, damage(&inventory)), (1, 1));
        worn += elytra.step(GLIDING, 2.0, &mut inventory, true).worn;
        assert_eq!((worn, damage(&inventory)), (3, 3));
        elytra.step(Flight::default(), 0.05, &mut inventory, true);
        for _ in 0..19 {
            elytra.step(GLIDING, 0.05, &mut inventory, true);
        }
        assert_eq!(damage(&inventory), 3);
        let mut creative = wearing(None);
        let mut flier = Elytra::default();
        assert_eq!(flier.step(GLIDING, 10.0, &mut creative, false).worn, 0);
        assert_eq!(damage(&creative), 0);
    }

    #[test]
    fn unbreaking_spares_its_share_of_the_wear() {
        let mut inventory = wearing(Some(json!({
            "minecraft:max_damage": 1000,
            "minecraft:enchantments": {"minecraft:unbreaking": 3}
        })));
        let mut elytra = Elytra::default();
        let worn = elytra.step(GLIDING, 200.0, &mut inventory, true).worn;
        assert!((30..75).contains(&worn), "{worn}");
        assert_eq!(damage(&inventory), u64::from(worn));
    }

    #[test]
    fn flying_into_a_wall_hurts_by_the_speed_lost_like_vanilla() {
        assert_eq!(wall_damage(1.5, 0.0), Some(12.0));
        assert_eq!(wall_damage(1.5, 1.25), None);
        assert_eq!(wall_damage(0.3, 0.0), None);
        assert!((wall_damage(0.5, 0.1).unwrap() - 1.0).abs() < 1e-6);
        let mut inventory = wearing(None);
        let mut elytra = Elytra::default();
        let fast = Flight {
            gliding: true,
            motion: [1.2, -0.3, 0.9],
            against_wall: true,
        };
        assert_eq!(
            elytra.step(fast, 0.01, &mut inventory, true).wall_damage,
            None
        );
        let stopped = Flight {
            gliding: true,
            motion: [0.0, -0.3, 0.9],
            against_wall: true,
        };
        let hit = elytra.step(stopped, 0.01, &mut inventory, true).wall_damage;
        assert!((hit.unwrap() - 3.0).abs() < 1e-5, "{hit:?}");
        let mut lander = Elytra::default();
        lander.step(fast, 0.01, &mut inventory, true);
        let landed = Flight {
            gliding: false,
            motion: [0.0; 3],
            against_wall: true,
        };
        assert_eq!(
            lander.step(landed, 0.01, &mut inventory, true).wall_damage,
            None
        );
    }

    #[test]
    fn speed_lost_away_from_a_wall_does_not_hurt() {
        let mut inventory = wearing(None);
        let mut elytra = Elytra::default();
        let fast = Flight {
            gliding: true,
            motion: [1.5, -0.3, 0.0],
            against_wall: false,
        };
        elytra.step(fast, 0.05, &mut inventory, true);
        let webbed = Flight {
            motion: [0.0; 3],
            ..fast
        };
        assert_eq!(
            elytra.step(webbed, 0.05, &mut inventory, true).wall_damage,
            None
        );
        elytra.step(fast, 0.05, &mut inventory, true);
        let walled = Flight {
            against_wall: true,
            ..webbed
        };
        let hit = elytra.step(walled, 0.05, &mut inventory, true).wall_damage;
        assert!((hit.unwrap() - 12.0).abs() < 1e-5, "{hit:?}");
    }

    #[test]
    fn a_slow_glide_counts_at_most_one_block_of_fall() {
        let slow = Flight {
            gliding: true,
            motion: [1.0, -0.4, 0.0],
            against_wall: false,
        };
        assert_eq!(fall_peak(Some(100.0), 60.0, slow), Some(61.0));
        assert_eq!(fall_peak(Some(60.5), 60.0, slow), Some(60.5));
        assert_eq!(fall_peak(None, 60.0, slow), None);
        let dive = Flight {
            gliding: true,
            motion: [1.0, -0.6, 0.0],
            against_wall: false,
        };
        assert_eq!(fall_peak(Some(100.0), 60.0, dive), Some(100.0));
        let falling = Flight {
            gliding: false,
            motion: [0.0, -0.1, 0.0],
            against_wall: false,
        };
        assert_eq!(fall_peak(Some(100.0), 60.0, falling), Some(100.0));
    }

    #[test]
    fn floating_restarts_the_fall_and_a_fall_after_it_hurts_from_where_it_ended() {
        let mut status = SurvivalStatus::default();
        assert!(!floating(&status));
        status.add_effect(EffectKind::Levitation, 200, 0);
        assert!(floating(&status));
        let mut peak = Some(64.0);
        for feet in [64.0, 66.0, 70.0, 72.9] {
            peak = float_peak(peak, feet, floating(&status));
            assert_eq!(peak, Some(feet));
        }
        status.remove_effect(EffectKind::Levitation);
        assert!(!floating(&status));
        for feet in [72.5, 70.0, 64.0] {
            peak = float_peak(peak, feet, floating(&status));
        }
        assert_eq!(peak, Some(72.9));
        let mut survival = crate::survival::Survival::default();
        survival.observe(20, 20);
        let fall = survival.impulse_fall(72.9 - 64.0, 64.0);
        assert!(survival.fall(fall, Some("minecraft:stone"), false) >= 5);
        status.add_effect(EffectKind::SlowFalling, 200, 0);
        assert!(floating(&status));
        assert_eq!(float_peak(Some(100.0), 60.0, floating(&status)), Some(60.0));
        assert_eq!(float_peak(None, 60.0, true), None);
    }
}
