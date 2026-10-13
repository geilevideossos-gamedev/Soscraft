use minecraftoss_player::inventory::Inventory;

use crate::offhand::OFFHAND;

pub const ITEM: &str = "minecraft:spyglass";
pub const USE_DURATION: u32 = 1200;
pub const ZOOM_FOV_MODIFIER: f32 = 0.1;
pub const USE_SOUND: &str = "minecraft:item.spyglass.use";
pub const STOP_SOUND: &str = "minecraft:item.spyglass.stop_using";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scoping {
    pub slot: usize,
    pub ticks: u32,
}

impl Scoping {
    pub fn start(inventory: &Inventory, slot: usize) -> Option<Self> {
        let stack = inventory.slots.get(slot)?.as_ref()?;
        (stack.id == ITEM).then_some(Self { slot, ticks: 0 })
    }

    pub fn holds_spyglass(&self, inventory: &Inventory, selected: usize) -> bool {
        (self.slot == OFFHAND || self.slot == selected)
            && inventory
                .slots
                .get(self.slot)
                .and_then(Option::as_ref)
                .is_some_and(|stack| stack.id == ITEM)
    }
}

pub fn step(
    scoping: Option<Scoping>,
    use_held: bool,
    alive: bool,
    inventory: &Inventory,
    selected: usize,
) -> (Option<Scoping>, bool) {
    let Some(scoping) = scoping.filter(|s| alive && s.holds_spyglass(inventory, selected)) else {
        return (None, false);
    };
    if use_held && scoping.ticks < USE_DURATION {
        (Some(scoping), false)
    } else {
        (None, true)
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
    fn only_a_spyglass_in_the_used_hand_starts_scoping() {
        assert_eq!(
            Scoping::start(&holding(3, ITEM), 3),
            Some(Scoping { slot: 3, ticks: 0 })
        );
        assert_eq!(Scoping::start(&holding(3, ITEM), 4), None);
        assert_eq!(Scoping::start(&holding(3, "minecraft:stick"), 3), None);
        assert_eq!(
            Scoping::start(&holding(OFFHAND, ITEM), OFFHAND),
            Some(Scoping {
                slot: OFFHAND,
                ticks: 0
            })
        );
    }

    #[test]
    fn letting_go_stops_with_the_sound_but_switching_away_or_dying_stops_quietly() {
        let inventory = holding(2, ITEM);
        let scoping = Some(Scoping { slot: 2, ticks: 30 });
        assert_eq!(step(scoping, true, true, &inventory, 2), (scoping, false));
        assert_eq!(step(scoping, false, true, &inventory, 2), (None, true));
        assert_eq!(step(scoping, true, true, &inventory, 5), (None, false));
        assert_eq!(step(scoping, true, false, &inventory, 2), (None, false));
        assert_eq!(
            step(scoping, true, true, &holding(2, "minecraft:stick"), 2),
            (None, false)
        );
        assert_eq!(step(None, true, true, &inventory, 2), (None, false));
        let offhand = Some(Scoping {
            slot: OFFHAND,
            ticks: 0,
        });
        assert_eq!(
            step(offhand, true, true, &holding(OFFHAND, ITEM), 7),
            (offhand, false)
        );
    }

    #[test]
    fn a_minute_of_scoping_finishes_the_use_like_letting_go() {
        let inventory = holding(0, ITEM);
        let almost = Some(Scoping {
            slot: 0,
            ticks: USE_DURATION - 1,
        });
        assert_eq!(step(almost, true, true, &inventory, 0), (almost, false));
        let done = Some(Scoping {
            slot: 0,
            ticks: USE_DURATION,
        });
        assert_eq!(step(done, true, true, &inventory, 0), (None, true));
    }
}
