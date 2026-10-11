use std::collections::HashSet;

use minecraftoss_player::inventory::{Inventory, ItemStack};

pub const WEAPON_PREFIX: &str = "iw4:weapon/";
pub const HOTBAR: usize = 9;

pub fn weapon_of(stack: &ItemStack) -> Option<u32> {
    stack.id.strip_prefix(WEAPON_PREFIX)?.parse().ok()
}

pub fn weapon_item(weapon: u32) -> ItemStack {
    let mut stack = ItemStack::new(format!("{WEAPON_PREFIX}{weapon}"), 1);
    stack.max = 1;
    stack
}

pub fn sync_weapons(inventory: &mut Inventory, owned: &[u32]) {
    let keep = |stack: &Option<ItemStack>| {
        stack
            .as_ref()
            .is_none_or(|s| weapon_of(s).is_none_or(|w| owned.contains(&w)))
    };
    for slot in inventory
        .slots
        .iter_mut()
        .chain(inventory.crafting.iter_mut())
        .chain(inventory.workbench.iter_mut())
    {
        if !keep(slot) {
            *slot = None;
        }
    }
    if !keep(&inventory.cursor) {
        inventory.cursor = None;
    }
    let held: HashSet<u32> = inventory
        .slots
        .iter()
        .chain(inventory.crafting.iter())
        .chain(inventory.workbench.iter())
        .chain(std::iter::once(&inventory.cursor))
        .filter_map(|s| s.as_ref().and_then(weapon_of))
        .collect();
    for &weapon in owned {
        if held.contains(&weapon) {
            continue;
        }
        let free = (0..HOTBAR)
            .chain(HOTBAR..36)
            .find(|&i| inventory.slots[i].is_none());
        if let Some(i) = free {
            inventory.slots[i] = Some(weapon_item(weapon));
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Follow {
    pending: Option<u32>,
    last_selected: Option<usize>,
}

impl Follow {
    pub fn request(
        &mut self,
        inventory: &Inventory,
        selected: &mut usize,
        held: u32,
    ) -> Option<u32> {
        let selected_gun = inventory.slots[*selected].as_ref().and_then(weapon_of);
        let changed = self.last_selected != Some(*selected);
        self.last_selected = Some(*selected);
        if self.pending == Some(held) {
            self.pending = None;
        }
        if changed {
            if let Some(gun) = selected_gun.filter(|&gun| gun != held) {
                self.pending = Some(gun);
            }
        } else if self.pending.is_none()
            && selected_gun.is_some_and(|gun| gun != held)
            && let Some(slot) =
                (0..HOTBAR).find(|&i| inventory.slots[i].as_ref().and_then(weapon_of) == Some(held))
        {
            *selected = slot;
            self.last_selected = Some(slot);
        }
        self.pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn guns_follow_the_mw2_weapons_and_a_lost_gun_leaves() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:dirt", 3));
        sync_weapons(&mut inventory, &[7, 9]);
        assert_eq!(inventory.slots[1].as_ref().and_then(weapon_of), Some(7));
        assert_eq!(inventory.slots[2].as_ref().and_then(weapon_of), Some(9));
        sync_weapons(&mut inventory, &[9]);
        assert!(inventory.slots[1].is_none());
        assert_eq!(inventory.slots[2].as_ref().and_then(weapon_of), Some(9));
        assert_eq!(inventory.slots[0].as_ref().map(|s| s.count), Some(3));
    }

    #[test]
    fn a_picked_gun_is_asked_for_and_an_mw2_switch_moves_the_selection() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(weapon_item(7));
        inventory.slots[1] = Some(weapon_item(9));
        let mut follow = Follow::default();
        let mut selected = 1;
        assert_eq!(follow.request(&inventory, &mut selected, 7), Some(9));
        assert_eq!(
            follow.request(&inventory, &mut selected, 7),
            Some(9),
            "still raising"
        );
        assert_eq!(follow.request(&inventory, &mut selected, 9), None);
        assert_eq!(selected, 1);
        assert_eq!(follow.request(&inventory, &mut selected, 7), None);
        assert_eq!(selected, 0, "MW2 raised the other gun on its own");
    }
}
