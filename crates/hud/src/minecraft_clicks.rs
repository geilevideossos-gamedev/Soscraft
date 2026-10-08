use frame::{McClick, McSlot, McStack};

use crate::minecraft_screens::{Group, SlotSpec};

const DOUBLE_CLICK: f64 = 0.25;

pub(crate) trait Menu {
    fn slots(&self) -> &[SlotSpec];
    fn stack(&self, slot: McSlot) -> Option<&McStack>;
    fn carried(&self) -> Option<&McStack>;
    fn capacity(&self, slot: McSlot) -> Option<u8>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Target {
    Slot(McSlot),
    Outside,
    Nothing,
}

impl Target {
    fn slot(self) -> Option<McSlot> {
        match self {
            Self::Slot(slot) => Some(slot),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Drag {
    pub button: usize,
    pub kind: u8,
    pub slots: Vec<McSlot>,
}

#[derive(Default)]
pub(crate) struct Clicks {
    last_slot: Option<McSlot>,
    last_press: Option<(f64, usize)>,
    double_click: bool,
    skip_release: bool,
    pub drag: Option<Drag>,
    last_quick_moved: Option<McStack>,
}

pub(crate) fn same_item(a: &McStack, b: &McStack) -> bool {
    a.id == b.id
        && a.weapon == b.weapon
        && a.damage == b.damage
        && a.name == b.name
        && a.named == b.named
        && a.enchantments == b.enchantments
}

pub(crate) fn quick_replace(slot: Option<&McStack>, carried: &McStack) -> bool {
    slot.is_none_or(|stack| same_item(stack, carried) && stack.count <= carried.max)
}

pub(crate) fn group_of(menu: &dyn Menu, slot: McSlot) -> Option<Group> {
    menu.slots()
        .iter()
        .find(|spec| spec.slot == slot)
        .map(|spec| spec.group)
}

pub(crate) fn takes_all(menu: &dyn Menu, slot: McSlot) -> bool {
    !matches!(group_of(menu, slot), Some(Group::Result | Group::Output))
}

pub(crate) fn place_count(slots: usize, kind: u8, carried: &McStack) -> u32 {
    match kind {
        0 => u32::from(carried.count) / slots.max(1) as u32,
        1 => 1,
        _ => u32::from(carried.max),
    }
}

pub(crate) fn preview(menu: &dyn Menu, drag: &Drag, slot: McSlot) -> Option<(u32, bool)> {
    let carried = menu.carried()?;
    if !drag.slots.contains(&slot) {
        return None;
    }
    let capacity = u32::from(menu.capacity(slot)?.min(carried.max));
    let existing = menu.stack(slot).map_or(0, |stack| u32::from(stack.count));
    let wanted = place_count(drag.slots.len(), drag.kind, carried) + existing;
    Some((wanted.min(capacity), wanted > capacity))
}

pub(crate) fn remainder(menu: &dyn Menu, drag: &Drag) -> u32 {
    let Some(carried) = menu.carried() else {
        return 0;
    };
    if drag.kind == 2 {
        return u32::from(carried.max);
    }
    let mut left = i64::from(carried.count);
    for &slot in &drag.slots {
        let existing = menu.stack(slot).map_or(0, |stack| i64::from(stack.count));
        if let Some((count, _)) = preview(menu, drag, slot) {
            left -= i64::from(count) - existing;
        }
    }
    left.max(0) as u32
}

impl Clicks {
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn press(
        &mut self,
        menu: &dyn Menu,
        target: Target,
        button: usize,
        shift: bool,
        now: f64,
    ) -> Vec<McClick> {
        let mut out = Vec::new();
        let hovered = target.slot();
        let repeat = self
            .last_press
            .is_some_and(|(at, last)| now - at < DOUBLE_CLICK && last == button);
        self.double_click = self.last_slot == hovered && repeat;
        self.last_press = Some((now, button));
        self.skip_release = false;
        if target != Target::Nothing && self.drag.is_none() {
            if menu.carried().is_none() {
                match target {
                    Target::Slot(slot) if shift => {
                        self.last_quick_moved = menu.stack(slot).cloned();
                        out.push(McClick::Slot {
                            slot,
                            right: button == 1,
                            shift: true,
                        });
                    }
                    Target::Slot(slot) => out.push(McClick::Slot {
                        slot,
                        right: button == 1,
                        shift: false,
                    }),
                    _ => {}
                }
                self.skip_release = true;
            } else {
                self.drag = Some(Drag {
                    button,
                    kind: if button == 1 { 1 } else { 0 },
                    slots: Vec::new(),
                });
            }
        }
        self.last_slot = hovered;
        out
    }

    pub(crate) fn drag_over(&mut self, menu: &dyn Menu, target: Target) {
        let (Some(slot), Some(carried)) = (target.slot(), menu.carried()) else {
            return;
        };
        let Some(drag) = self.drag.as_mut() else {
            return;
        };
        let eligible = (usize::from(carried.count) > drag.slots.len() || drag.kind == 2)
            && quick_replace(menu.stack(slot), carried)
            && menu.capacity(slot).is_some();
        if eligible && !drag.slots.contains(&slot) {
            drag.slots.push(slot);
        }
    }

    pub(crate) fn release(
        &mut self,
        menu: &dyn Menu,
        target: Target,
        button: usize,
        shift: bool,
    ) -> Vec<McClick> {
        let mut out = Vec::new();
        let hovered = target.slot();
        if let Some(slot) =
            hovered.filter(|&slot| self.double_click && button == 0 && takes_all(menu, slot))
        {
            if shift {
                if let Some(moved) = self.last_quick_moved.clone() {
                    let group = group_of(menu, slot);
                    for spec in menu.slots() {
                        let has = menu.stack(spec.slot);
                        if has.is_some() && Some(spec.group) == group && quick_replace(has, &moved)
                        {
                            out.push(McClick::Slot {
                                slot: spec.slot,
                                right: false,
                                shift: true,
                            });
                        }
                    }
                }
            } else if menu.carried().is_some() && menu.stack(slot).is_none() {
                out.push(McClick::Gather { slot, right: false });
            }
            self.double_click = false;
            self.drag = None;
            return out;
        }
        if self.drag.as_ref().is_some_and(|drag| drag.button != button) {
            self.drag = None;
            self.skip_release = true;
            return out;
        }
        if self.skip_release {
            self.skip_release = false;
            return out;
        }
        if let Some(drag) = self.drag.take().filter(|drag| !drag.slots.is_empty()) {
            let right = drag.kind == 1;
            if drag.slots.len() == 1 {
                out.push(McClick::Slot {
                    slot: drag.slots[0],
                    right,
                    shift: false,
                });
            } else {
                out.push(McClick::Spread {
                    slots: drag.slots,
                    right,
                });
            }
            return out;
        }
        self.drag = None;
        if menu.carried().is_some() {
            match target {
                Target::Slot(slot) if shift => {
                    self.last_quick_moved = menu.stack(slot).cloned();
                    out.push(McClick::Slot {
                        slot,
                        right: button == 1,
                        shift: true,
                    });
                }
                Target::Slot(slot) => out.push(McClick::Slot {
                    slot,
                    right: button == 1,
                    shift: false,
                }),
                Target::Outside => out.push(McClick::Outside { right: button == 1 }),
                Target::Nothing => {}
            }
        }
        out
    }

    pub(crate) fn key(
        &self,
        menu: &dyn Menu,
        hovered: Option<McSlot>,
        hotbar: Option<usize>,
        throw: Option<bool>,
    ) -> Vec<McClick> {
        let mut out = Vec::new();
        let Some(slot) = hovered else { return out };
        if let Some(hotbar) = hotbar.filter(|_| menu.carried().is_none()) {
            out.push(McClick::Swap { slot, hotbar });
        }
        if let Some(whole) = throw.filter(|_| menu.stack(slot).is_some()) {
            out.push(McClick::Throw { slot, whole });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestMenu {
        specs: Vec<SlotSpec>,
        stacks: Vec<(McSlot, McStack)>,
        carried: Option<McStack>,
        refuse: Vec<McSlot>,
    }

    impl Menu for TestMenu {
        fn slots(&self) -> &[SlotSpec] {
            &self.specs
        }
        fn stack(&self, slot: McSlot) -> Option<&McStack> {
            self.stacks
                .iter()
                .find(|(s, _)| *s == slot)
                .map(|(_, stack)| stack)
        }
        fn carried(&self) -> Option<&McStack> {
            self.carried.as_ref()
        }
        fn capacity(&self, slot: McSlot) -> Option<u8> {
            let accepted = !self.refuse.contains(&slot)
                && self.specs.iter().any(|spec| {
                    spec.slot == slot && !matches!(spec.group, Group::Result | Group::Output)
                });
            accepted.then_some(64)
        }
    }

    fn stack(id: &str, count: u8) -> McStack {
        McStack {
            id: id.into(),
            count,
            weapon: None,
            damage: None,
            max: 64,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint: false,
        }
    }

    fn spec(slot: McSlot, group: Group) -> SlotSpec {
        SlotSpec {
            slot,
            x: 0,
            y: 0,
            group,
            empty: None,
        }
    }

    fn menu() -> TestMenu {
        let mut specs = vec![spec(McSlot::Result, Group::Result)];
        specs.extend((0..4).map(|i| spec(McSlot::Crafting(i), Group::Grid)));
        specs.extend((0..36).map(|i| spec(McSlot::Inventory(i), Group::Player)));
        TestMenu {
            specs,
            stacks: Vec::new(),
            carried: None,
            refuse: Vec::new(),
        }
    }

    const A: McSlot = McSlot::Inventory(9);
    const B: McSlot = McSlot::Inventory(10);
    const C: McSlot = McSlot::Inventory(11);

    #[test]
    fn a_click_with_an_empty_cursor_picks_up_on_press_and_skips_the_release() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.stacks.push((A, stack("minecraft:dirt", 10)));
        assert_eq!(
            clicks.press(&menu, Target::Slot(A), 0, false, 0.0),
            vec![McClick::Slot {
                slot: A,
                right: false,
                shift: false
            }]
        );
        menu.carried = Some(stack("minecraft:dirt", 10));
        menu.stacks.clear();
        assert!(clicks.release(&menu, Target::Slot(A), 0, false).is_empty());
    }

    #[test]
    fn a_click_with_a_carried_stack_places_on_release() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 10));
        assert!(
            clicks
                .press(&menu, Target::Slot(B), 1, false, 0.0)
                .is_empty()
        );
        assert_eq!(
            clicks.release(&menu, Target::Slot(B), 1, false),
            vec![McClick::Slot {
                slot: B,
                right: true,
                shift: false
            }]
        );
    }

    #[test]
    fn dragging_splits_evenly_and_one_slot_is_a_plain_click() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 10));
        clicks.press(&menu, Target::Slot(A), 0, false, 0.0);
        clicks.drag_over(&menu, Target::Slot(A));
        clicks.drag_over(&menu, Target::Slot(B));
        clicks.drag_over(&menu, Target::Slot(B));
        clicks.drag_over(&menu, Target::Slot(McSlot::Result));
        let drag = clicks.drag.clone().unwrap();
        assert_eq!(drag.slots, vec![A, B]);
        assert_eq!(preview(&menu, &drag, A), Some((5, false)));
        assert_eq!(remainder(&menu, &drag), 0);
        assert_eq!(
            clicks.release(&menu, Target::Slot(B), 0, false),
            vec![McClick::Spread {
                slots: vec![A, B],
                right: false
            }]
        );
        clicks.press(&menu, Target::Slot(A), 1, false, 5.0);
        clicks.drag_over(&menu, Target::Slot(A));
        assert_eq!(
            clicks.release(&menu, Target::Slot(C), 1, false),
            vec![McClick::Slot {
                slot: A,
                right: true,
                shift: false
            }]
        );
    }

    #[test]
    fn a_drag_takes_no_more_slots_than_items() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 2));
        menu.stacks.push((C, stack("minecraft:stone", 1)));
        clicks.press(&menu, Target::Slot(A), 0, false, 0.0);
        for slot in [C, A, B, McSlot::Inventory(12)] {
            clicks.drag_over(&menu, Target::Slot(slot));
        }
        assert_eq!(clicks.drag.as_ref().unwrap().slots, vec![A, B]);
    }

    #[test]
    fn capped_previews_turn_yellow() {
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 64));
        menu.stacks.push((A, stack("minecraft:dirt", 60)));
        let drag = Drag {
            button: 0,
            kind: 0,
            slots: vec![A, B],
        };
        assert_eq!(preview(&menu, &drag, A), Some((64, true)));
        assert_eq!(preview(&menu, &drag, B), Some((32, false)));
        assert_eq!(remainder(&menu, &drag), 64 - 4 - 32);
    }

    #[test]
    fn another_button_cancels_a_drag() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 10));
        clicks.press(&menu, Target::Slot(A), 0, false, 0.0);
        clicks.drag_over(&menu, Target::Slot(A));
        clicks.drag_over(&menu, Target::Slot(B));
        assert!(clicks.release(&menu, Target::Slot(B), 1, false).is_empty());
        assert!(clicks.drag.is_none());
        assert!(clicks.release(&menu, Target::Slot(B), 0, false).is_empty());
    }

    #[test]
    fn a_double_click_gathers_on_release_but_never_from_a_result() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.stacks.push((A, stack("minecraft:dirt", 10)));
        clicks.press(&menu, Target::Slot(A), 0, false, 1.0);
        menu.stacks.clear();
        menu.carried = Some(stack("minecraft:dirt", 10));
        clicks.release(&menu, Target::Slot(A), 0, false);
        assert!(
            clicks
                .press(&menu, Target::Slot(A), 0, false, 1.2)
                .is_empty()
        );
        assert_eq!(
            clicks.release(&menu, Target::Slot(A), 0, false),
            vec![McClick::Gather {
                slot: A,
                right: false
            }]
        );
        clicks.press(&menu, Target::Slot(McSlot::Result), 0, false, 2.0);
        clicks.press(&menu, Target::Slot(McSlot::Result), 0, false, 2.1);
        assert_eq!(
            clicks.release(&menu, Target::Slot(McSlot::Result), 0, false),
            vec![McClick::Slot {
                slot: McSlot::Result,
                right: false,
                shift: false
            }]
        );
    }

    #[test]
    fn a_slow_second_click_is_not_a_double_click() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 10));
        clicks.press(&menu, Target::Slot(A), 0, false, 1.0);
        clicks.release(&menu, Target::Slot(A), 0, false);
        clicks.press(&menu, Target::Slot(A), 0, false, 1.3);
        assert_eq!(
            clicks.release(&menu, Target::Slot(A), 0, false),
            vec![McClick::Slot {
                slot: A,
                right: false,
                shift: false
            }]
        );
    }

    #[test]
    fn shift_double_click_moves_every_match_in_the_same_container() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.stacks.push((A, stack("minecraft:dirt", 64)));
        menu.stacks.push((C, stack("minecraft:dirt", 3)));
        menu.stacks
            .push((McSlot::Crafting(0), stack("minecraft:dirt", 3)));
        menu.stacks
            .push((McSlot::Inventory(12), stack("minecraft:stone", 3)));
        assert_eq!(
            clicks.press(&menu, Target::Slot(A), 0, true, 0.0),
            vec![McClick::Slot {
                slot: A,
                right: false,
                shift: true
            }]
        );
        clicks.release(&menu, Target::Slot(A), 0, true);
        clicks.press(&menu, Target::Slot(A), 0, true, 0.1);
        let moved = clicks.release(&menu, Target::Slot(A), 0, true);
        assert_eq!(
            moved,
            vec![
                McClick::Slot {
                    slot: A,
                    right: false,
                    shift: true
                },
                McClick::Slot {
                    slot: C,
                    right: false,
                    shift: true
                }
            ]
        );
    }

    #[test]
    fn clicking_outside_drops_and_inside_the_frame_does_nothing() {
        let mut clicks = Clicks::default();
        let mut menu = menu();
        menu.carried = Some(stack("minecraft:dirt", 10));
        clicks.press(&menu, Target::Outside, 1, false, 0.0);
        assert_eq!(
            clicks.release(&menu, Target::Outside, 1, false),
            vec![McClick::Outside { right: true }]
        );
        clicks.press(&menu, Target::Nothing, 0, false, 5.0);
        assert!(clicks.release(&menu, Target::Nothing, 0, false).is_empty());
    }

    #[test]
    fn number_keys_swap_only_with_an_empty_cursor_and_q_throws_a_stack() {
        let clicks = Clicks::default();
        let mut menu = menu();
        menu.stacks.push((A, stack("minecraft:dirt", 10)));
        assert_eq!(
            clicks.key(&menu, Some(A), Some(3), None),
            vec![McClick::Swap { slot: A, hotbar: 3 }]
        );
        assert_eq!(
            clicks.key(&menu, Some(A), Some(40), None),
            vec![McClick::Swap {
                slot: A,
                hotbar: 40
            }]
        );
        assert_eq!(
            clicks.key(&menu, Some(A), None, Some(true)),
            vec![McClick::Throw {
                slot: A,
                whole: true
            }]
        );
        assert!(clicks.key(&menu, Some(B), None, Some(false)).is_empty());
        menu.carried = Some(stack("minecraft:stone", 1));
        assert!(clicks.key(&menu, Some(A), Some(3), None).is_empty());
        assert!(clicks.key(&menu, None, Some(3), None).is_empty());
    }
}
