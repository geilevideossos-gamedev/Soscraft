use minecraft_terrain::trident;
use minecraftoss_player::inventory::Inventory;

use crate::offhand::OFFHAND;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Charging {
    pub slot: usize,
    pub ticks: u32,
}

impl Charging {
    pub fn start(inventory: &Inventory, slot: usize, wet: bool) -> Option<Self> {
        let stack = inventory.slots.get(slot)?.as_ref()?;
        trident::can_start(inventory, stack, wet).then_some(Self { slot, ticks: 0 })
    }

    pub fn holds_trident(&self, inventory: &Inventory, selected: usize) -> bool {
        (self.slot == OFFHAND || self.slot == selected)
            && inventory
                .slots
                .get(self.slot)
                .and_then(Option::as_ref)
                .is_some_and(|stack| stack.id == trident::ITEM)
    }
}

pub fn released(
    charging: Option<Charging>,
    use_held: bool,
    alive: bool,
    inventory: &Inventory,
    selected: usize,
) -> (Option<Charging>, Option<Charging>) {
    let Some(charging) = charging.filter(|c| alive && c.holds_trident(inventory, selected)) else {
        return (None, None);
    };
    if use_held {
        (Some(charging), None)
    } else {
        (None, Some(charging))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::inventory::ItemStack;

    fn holding(slot: usize, id: &str) -> Inventory {
        let mut inventory = Inventory::default();
        inventory.slots[slot] = Some(ItemStack::new(id, 1));
        inventory
    }

    #[test]
    fn only_a_trident_starts_charging_and_a_riptide_one_only_when_wet() {
        let plain = holding(2, trident::ITEM);
        assert_eq!(
            Charging::start(&plain, 2, false),
            Some(Charging { slot: 2, ticks: 0 })
        );
        assert_eq!(Charging::start(&plain, 3, false), None);
        assert_eq!(
            Charging::start(&holding(2, "minecraft:stick"), 2, false),
            None
        );
        let mut riptide = holding(OFFHAND, trident::ITEM);
        riptide.slots[OFFHAND].as_mut().unwrap().components = Some(serde_json::json!({
            "minecraft:enchantments": {"minecraft:riptide": 2}
        }));
        assert_eq!(Charging::start(&riptide, OFFHAND, false), None);
        assert!(Charging::start(&riptide, OFFHAND, true).is_some());
    }

    #[test]
    fn letting_go_releases_but_switching_away_or_dying_only_stops() {
        let inventory = holding(4, trident::ITEM);
        let charging = Some(Charging { slot: 4, ticks: 12 });
        assert_eq!(
            released(charging, true, true, &inventory, 4),
            (charging, None)
        );
        assert_eq!(
            released(charging, false, true, &inventory, 4),
            (None, charging)
        );
        assert_eq!(released(charging, false, true, &inventory, 5), (None, None));
        assert_eq!(
            released(charging, false, false, &inventory, 4),
            (None, None)
        );
        assert_eq!(
            released(charging, false, true, &Inventory::default(), 4),
            (None, None)
        );
        assert_eq!(released(None, false, true, &inventory, 4), (None, None));
        let offhand = Some(Charging {
            slot: OFFHAND,
            ticks: 3,
        });
        assert_eq!(
            released(offhand, false, true, &holding(OFFHAND, trident::ITEM), 0),
            (None, offhand)
        );
    }
}
