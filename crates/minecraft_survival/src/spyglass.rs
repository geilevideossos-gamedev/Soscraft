use minecraft_net::messages::{Hand, UsingView};
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

pub fn seen_scoping(using: Option<UsingView>, inventory: &Inventory, selected: usize) -> bool {
    using.is_some_and(|using| {
        let slot = match using.hand {
            Hand::Main => selected,
            Hand::Off => OFFHAND,
        };
        inventory
            .slots
            .get(slot)
            .and_then(Option::as_ref)
            .is_some_and(|stack| stack.id == ITEM)
    })
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

    #[test]
    fn a_friend_sees_scoping_only_when_the_used_hand_holds_a_spyglass() {
        let main = Some(UsingView {
            hand: Hand::Main,
            ticks: 4,
            kind: minecraft_net::messages::UseKind::Scope,
        });
        let off = Some(UsingView {
            hand: Hand::Off,
            ticks: 4,
            kind: minecraft_net::messages::UseKind::Scope,
        });
        assert!(seen_scoping(main, &holding(2, ITEM), 2));
        assert!(!seen_scoping(main, &holding(2, ITEM), 3));
        assert!(!seen_scoping(main, &holding(2, "minecraft:bread"), 2));
        assert!(!seen_scoping(None, &holding(2, ITEM), 2));
        assert!(seen_scoping(off, &holding(OFFHAND, ITEM), 2));
        let mut eating_beside_a_spyglass = holding(2, ITEM);
        eating_beside_a_spyglass.slots[OFFHAND] = Some(ItemStack::new("minecraft:bread", 1));
        assert!(!seen_scoping(off, &eating_beside_a_spyglass, 2));
    }
}
