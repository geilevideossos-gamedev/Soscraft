use glam::DVec3;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_player::crafting::RecipeBook;
use minecraftoss_player::inventory::{Inventory, ItemStack};

use crate::containers::pick;
use crate::crafting::move_into;

pub const SADDLE: usize = 0;
pub const BODY: usize = 1;
pub const CHEST: usize = 2;
pub const REACH: f64 = 7.0;
const HOTBAR: usize = 9;
const STORAGE: usize = 36;

#[derive(Clone, Debug, PartialEq)]
pub struct MountInventory {
    pub vehicle: u64,
    pub columns: usize,
    pub saddle: bool,
    pub armor: bool,
    pub llama: bool,
    pub cells: Vec<Option<ItemStack>>,
}

fn is_carpet(id: &str) -> bool {
    id.ends_with("_carpet") && !id.contains("moss")
}

impl MountInventory {
    pub fn of(world: &EntityWorld, vehicle: u64, recipes: &RecipeBook) -> Option<Self> {
        let entity = world.cows().iter().find(|e| e.id == vehicle)?;
        let horse = entity.horse.as_ref()?;
        let stack = |id: &str| ItemStack {
            id: id.to_owned(),
            count: 1,
            max: recipes.max_stack(id).max(1),
            components: None,
        };
        let usable = entity.cow.health > 0.0 && !entity.cow.age.baby() && horse.tame;
        let columns = horse.inventory_columns().max(0) as usize;
        let mut cells = vec![
            horse.saddled.then(|| stack("minecraft:saddle")),
            horse.body_item.as_deref().map(stack),
        ];
        cells.extend(horse.inventory.iter().cloned());
        cells.resize(CHEST + columns * 3, None);
        Some(Self {
            vehicle,
            columns,
            saddle: usable && horse.kind.saddleable(),
            armor: horse.kind.wears_armor() || horse.kind.is_llama(),
            llama: horse.kind.is_llama(),
            cells,
        })
    }

    pub fn accepts(&self, index: usize, stack: &ItemStack) -> bool {
        match index {
            SADDLE => self.saddle && stack.id == "minecraft:saddle",
            BODY if self.llama => self.armor && is_carpet(&stack.id),
            BODY => self.armor && stack.id.ends_with("_horse_armor"),
            _ => index < self.cells.len(),
        }
    }

    pub fn capacity(&self, index: usize, stack: &ItemStack) -> Option<u8> {
        self.accepts(index, stack).then(|| self.limit(index))
    }

    fn limit(&self, index: usize) -> u8 {
        if index < CHEST { 1 } else { 99 }
    }

    pub fn saddled(&self) -> bool {
        self.cells.get(SADDLE).is_some_and(Option::is_some)
    }

    pub fn body(&self) -> Option<String> {
        self.cells
            .get(BODY)
            .and_then(Option::as_ref)
            .map(|stack| stack.id.clone())
    }

    pub fn items(&self) -> Vec<Option<ItemStack>> {
        self.cells.get(CHEST..).map_or_else(Vec::new, <[_]>::to_vec)
    }

    pub fn click(&mut self, inventory: &mut Inventory, index: usize, right: bool, shift: bool) {
        if index >= self.cells.len() {
            return;
        }
        if shift {
            if let Some(mut stack) = self.cells[index].take() {
                let order: Vec<usize> = (0..HOTBAR).rev().chain((HOTBAR..STORAGE).rev()).collect();
                move_into(&mut inventory.slots, &mut stack, &order);
                self.cells[index] = (stack.count > 0).then_some(stack);
            }
            return;
        }
        let may_place = inventory
            .cursor
            .as_ref()
            .is_some_and(|carried| self.accepts(index, carried));
        let limit = self.limit(index);
        pick(
            &mut self.cells[index],
            &mut inventory.cursor,
            right,
            may_place,
            limit,
        );
    }

    fn put_one(&mut self, index: usize, stack: &mut ItemStack) -> bool {
        if !self.accepts(index, stack) || self.cells[index].is_some() {
            return false;
        }
        self.cells[index] = Some(ItemStack {
            count: 1,
            ..stack.clone()
        });
        stack.count -= 1;
        true
    }

    pub fn quick_move_from_player(&mut self, inventory: &mut Inventory, slot: usize) {
        if slot >= STORAGE {
            return;
        }
        let Some(mut stack) = inventory.slots[slot].take() else {
            return;
        };
        let moved = self.put_one(BODY, &mut stack)
            || self.put_one(SADDLE, &mut stack)
            || self.columns > 0 && {
                let before = stack.count;
                let chest: Vec<usize> = (CHEST..self.cells.len()).collect();
                move_into(&mut self.cells, &mut stack, &chest);
                stack.count < before
            };
        if !moved {
            let order: Vec<usize> = if slot < HOTBAR {
                (HOTBAR..STORAGE).collect()
            } else {
                (0..HOTBAR).collect()
            };
            move_into(&mut inventory.slots, &mut stack, &order);
        }
        if stack.count > 0 {
            inventory.slots[slot] = Some(stack);
        }
    }

    pub fn swap(&mut self, inventory: &mut Inventory, index: usize, hotbar: usize) {
        if index >= self.cells.len() || hotbar >= inventory.slots.len() {
            return;
        }
        match (inventory.slots[hotbar].clone(), self.cells[index].is_some()) {
            (None, false) => {}
            (None, true) => inventory.slots[hotbar] = self.cells[index].take(),
            (Some(mut held), false) => {
                if !self.accepts(index, &held) {
                    return;
                }
                let moved = held.count.min(self.limit(index));
                self.cells[index] = Some(ItemStack {
                    count: moved,
                    ..held.clone()
                });
                held.count -= moved;
                inventory.slots[hotbar] = (held.count > 0).then_some(held);
            }
            (Some(held), true) => {
                if self.accepts(index, &held) && held.count <= self.limit(index) {
                    std::mem::swap(&mut self.cells[index], &mut inventory.slots[hotbar]);
                }
            }
        }
    }

    pub fn throw(&mut self, index: usize, whole: bool) -> Option<ItemStack> {
        let cell = self.cells.get_mut(index)?;
        let stack = cell.as_mut()?;
        let count = if whole { stack.count } else { 1 };
        let thrown = ItemStack {
            count,
            ..stack.clone()
        };
        stack.count -= count;
        if stack.count == 0 {
            *cell = None;
        }
        Some(thrown)
    }

    pub fn still_valid(&self, world: &EntityWorld, eye: DVec3) -> bool {
        let Some(entity) = world.cows().iter().find(|e| e.id == self.vehicle) else {
            return false;
        };
        let Some(horse) = entity.horse.as_ref() else {
            return false;
        };
        let body = &entity.cow.body;
        let half = f64::from(body.width) / 2.0;
        let min = body.position - DVec3::new(half, 0.0, half);
        let max = body.position + DVec3::new(half, f64::from(body.height), half);
        entity.cow.health > 0.0
            && horse.inventory_columns().max(0) as usize == self.columns
            && eye.clamp(min, max).distance_squared(eye) <= REACH * REACH
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::cow::Cow;
    use minecraftoss_entities::horse::{HorseKind, HorseState};

    fn equine(world: &mut EntityWorld, kind: HorseKind, tame: bool, chest: bool) -> u64 {
        let mut horse = HorseState::new(kind);
        horse.tame = tame;
        horse.chest = chest;
        if kind.is_llama() {
            horse.set_strength(2);
        }
        horse.resize_inventory();
        let mut cow = Cow::new(DVec3::new(0.5, 64.0, 0.5));
        cow.health = 20.0;
        world.spawn_horse(cow, horse, true)
    }

    fn stack(id: &str, count: u8) -> ItemStack {
        ItemStack {
            id: id.to_owned(),
            count,
            max: 64,
            components: None,
        }
    }

    #[test]
    fn a_horse_takes_a_saddle_and_armour_a_llama_a_carpet_and_chests_hold_anything() {
        let recipes = RecipeBook::default();
        let mut world = EntityWorld::default();
        let horse = equine(&mut world, HorseKind::Horse, true, false);
        let wild = equine(&mut world, HorseKind::Horse, false, false);
        let donkey = equine(&mut world, HorseKind::Donkey, true, true);
        let llama = equine(&mut world, HorseKind::Llama, true, true);
        let h = MountInventory::of(&world, horse, &recipes).unwrap();
        assert_eq!(
            (h.columns, h.cells.len(), h.saddle, h.armor),
            (0, 2, true, true)
        );
        assert!(h.accepts(SADDLE, &stack("minecraft:saddle", 1)));
        assert!(!h.accepts(SADDLE, &stack("minecraft:iron_horse_armor", 1)));
        assert!(h.accepts(BODY, &stack("minecraft:diamond_horse_armor", 1)));
        assert!(!h.accepts(BODY, &stack("minecraft:white_carpet", 1)));
        assert!(
            !MountInventory::of(&world, wild, &recipes).unwrap().saddle,
            "a wild horse's saddle slot is off"
        );
        let d = MountInventory::of(&world, donkey, &recipes).unwrap();
        assert_eq!(
            (d.columns, d.cells.len(), d.saddle, d.armor),
            (5, 17, true, false)
        );
        assert!(!d.accepts(BODY, &stack("minecraft:iron_horse_armor", 1)));
        assert!(d.accepts(16, &stack("minecraft:cobblestone", 64)));
        let l = MountInventory::of(&world, llama, &recipes).unwrap();
        assert_eq!(
            (l.columns, l.cells.len(), l.saddle, l.armor, l.llama),
            (2, 8, false, true, true)
        );
        assert!(l.accepts(BODY, &stack("minecraft:red_carpet", 1)));
        assert!(!l.accepts(BODY, &stack("minecraft:moss_carpet", 1)));
        assert!(!l.accepts(SADDLE, &stack("minecraft:saddle", 1)));
    }

    #[test]
    fn clicks_and_shift_clicks_fill_the_saddle_armour_and_chest_slots_like_vanilla() {
        let recipes = RecipeBook::default();
        let mut world = EntityWorld::default();
        let donkey = equine(&mut world, HorseKind::Donkey, true, true);
        let mut d = MountInventory::of(&world, donkey, &recipes).unwrap();
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(stack("minecraft:saddle", 2));
        inventory.slots[9] = Some(stack("minecraft:cobblestone", 64));
        inventory.slots[10] = Some(stack("minecraft:iron_horse_armor", 1));
        d.quick_move_from_player(&mut inventory, 0);
        assert!(d.saddled());
        assert_eq!(
            inventory.slots[0].as_ref().map(|s| s.count),
            Some(1),
            "one saddle goes on"
        );
        d.quick_move_from_player(&mut inventory, 9);
        assert_eq!(d.cells[CHEST].as_ref().map(|s| s.count), Some(64));
        assert!(inventory.slots[9].is_none());
        d.quick_move_from_player(&mut inventory, 10);
        assert_eq!(
            d.cells[CHEST + 1].as_ref().map(|s| s.id.as_str()),
            Some("minecraft:iron_horse_armor"),
            "a donkey wears no armour, so it goes in the chest"
        );
        d.click(&mut inventory, SADDLE, false, true);
        assert!(!d.saddled());
        assert_eq!(
            inventory.slots[0].as_ref().map(|s| s.count),
            Some(2),
            "shift-clicked back to the hotbar"
        );
        inventory.cursor = Some(stack("minecraft:cobblestone", 3));
        d.click(&mut inventory, SADDLE, false, false);
        assert!(
            !d.saddled() && inventory.cursor.is_some(),
            "only a saddle fits the saddle slot"
        );
        inventory.cursor = Some(stack("minecraft:saddle", 2));
        d.click(&mut inventory, SADDLE, false, false);
        assert_eq!(
            (d.saddled(), inventory.cursor.as_ref().map(|s| s.count)),
            (true, Some(1))
        );
        assert_eq!(d.throw(CHEST, false).map(|s| s.count), Some(1));
        assert_eq!(d.cells[CHEST].as_ref().map(|s| s.count), Some(63));
        inventory.slots[3] = None;
        d.swap(&mut inventory, CHEST, 3);
        assert_eq!(inventory.slots[3].as_ref().map(|s| s.count), Some(63));
        assert_eq!((d.items().len(), d.body()), (15, None));
    }

    #[test]
    fn the_screen_closes_when_the_mount_dies_moves_away_or_gets_a_chest() {
        let recipes = RecipeBook::default();
        let mut world = EntityWorld::default();
        let mule = equine(&mut world, HorseKind::Mule, true, false);
        let m = MountInventory::of(&world, mule, &recipes).unwrap();
        let eye = DVec3::new(2.5, 65.6, 0.5);
        assert!(m.still_valid(&world, eye));
        assert!(!m.still_valid(&world, DVec3::new(9.5, 65.6, 0.5)));
        let horse = world.cow_mut(mule).unwrap().horse.as_mut().unwrap();
        horse.chest = true;
        horse.resize_inventory();
        assert!(
            !m.still_valid(&world, eye),
            "a new chest changes the inventory"
        );
        let m = MountInventory::of(&world, mule, &recipes).unwrap();
        world.cow_mut(mule).unwrap().cow.health = 0.0;
        assert!(!m.still_valid(&world, eye));
    }
}
