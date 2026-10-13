use std::collections::HashSet;

use minecraft_terrain::fork_items::{self, Kind};
use minecraft_terrain::game_mode::PlayerMode;
use minecraftoss_player::inventory::{Inventory, ItemStack};
use serde_json::{Map, Value};

use crate::gun_names::{GunContent, GunNames};

pub const WEAPON_PREFIX: &str = "iw4:weapon/";
pub const HOTBAR: usize = 9;
pub const WEAPON_SLOTS: usize = 15;
pub const NO_AMMO: u8 = u8::MAX;
pub const AMMO_TYPES: usize = fork_items::AMMO_TYPES.len() + 1;
pub const TOO_MANY_GUNS: &str = "Too many guns on the hotbar";

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
    last_gun: Option<u32>,
}

impl Follow {
    pub fn request(
        &mut self,
        inventory: &Inventory,
        selected: &mut usize,
        held: u32,
    ) -> Option<u32> {
        self.request_with(inventory, &[None; HOTBAR], selected, held)
    }

    pub fn request_with(
        &mut self,
        inventory: &Inventory,
        guns: &[Option<u32>; HOTBAR],
        selected: &mut usize,
        held: u32,
    ) -> Option<u32> {
        let gun_at = |slot: usize| {
            inventory.slots[slot]
                .as_ref()
                .and_then(weapon_of)
                .or(guns.get(slot).copied().flatten())
        };
        let selected_gun = gun_at(*selected);
        let fork_gun = guns.get(*selected).copied().flatten();
        let changed = self.last_selected != Some(*selected) || self.last_gun != fork_gun;
        self.last_selected = Some(*selected);
        self.last_gun = fork_gun;
        if self.pending == Some(held) {
            self.pending = None;
        }
        if changed {
            if let Some(gun) = selected_gun.filter(|&gun| gun != held) {
                self.pending = Some(gun);
            }
        } else if self.pending.is_none()
            && selected_gun.is_some_and(|gun| gun != held)
            && let Some(slot) = (0..HOTBAR).find(|&i| gun_at(i) == Some(held))
        {
            *selected = slot;
            self.last_selected = Some(slot);
        }
        self.pending
    }
}

pub fn is_fork_gun(stack: &ItemStack) -> bool {
    fork_items::get(&stack.id).is_some_and(|row| row.kind == Kind::Gun)
}

pub fn is_gun(stack: &ItemStack) -> bool {
    weapon_of(stack).is_some() || is_fork_gun(stack)
}

pub use fork_items::{LETHAL_SLOT, TACTICAL_SLOT};
pub const BODY_SLOT: usize = 41;
pub const SADDLE_SLOT: usize = 42;
pub const GRENADE_STACK: u8 = 64;

pub fn fits(slot: usize, stack: &ItemStack) -> bool {
    match slot {
        BODY_SLOT | SADDLE_SLOT => false,
        LETHAL_SLOT | TACTICAL_SLOT => {
            fork_items::grenade_slot(&stack.id) == Some(slot) && stack.count <= GRENADE_STACK
        }
        _ => true,
    }
}

pub fn grenade_quick_move(inventory: &mut Inventory, index: usize) -> bool {
    if index >= crate::player_nbt::STORAGE {
        return false;
    }
    let Some(source) = inventory.slots[index].as_ref() else {
        return false;
    };
    let Some(target) = fork_items::grenade_slot(&source.id) else {
        return false;
    };
    let room = match inventory.slots.get(target) {
        Some(None) => source.max.min(GRENADE_STACK),
        Some(Some(held)) if held.same_item(source) => held.max.saturating_sub(held.count),
        _ => 0,
    };
    if room == 0 {
        return false;
    }
    let Some(mut source) = inventory.slots[index].take() else {
        return false;
    };
    let moved = room.min(source.count);
    match &mut inventory.slots[target] {
        Some(held) => held.count += moved,
        empty => {
            *empty = Some(ItemStack {
                count: moved,
                ..source.clone()
            })
        }
    }
    source.count -= moved;
    if source.count > 0 {
        inventory.slots[index] = Some(source);
    }
    true
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gun {
    pub weapon: u32,
    pub clip: i32,
    pub clip_left: i32,
    pub alt_clip: i32,
    pub ammo: u8,
    pub rebind: bool,
    pub alt_ammo: u8,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arsenal {
    pub guns: Vec<Gun>,
    pub selected: Option<u32>,
    pub stock: [i32; AMMO_TYPES],
    pub lethal: Option<(u32, i32)>,
    pub tactical: Option<(u32, i32)>,
    pub endless: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spent {
    Rounds {
        ammo: u8,
        rounds: i32,
    },
    Grenade {
        weapon: u32,
        delta: i32,
    },
    Clip {
        weapon: u32,
        clip: i32,
        clip_left: i32,
        alt_clip: i32,
    },
    Flare {
        pos: [f32; 3],
    },
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Binding {
    bound: Vec<(u32, usize, [i64; 3])>,
    guns: [Option<u32>; HOTBAR],
    inert_selected: bool,
}

impl Binding {
    pub fn guns(&self) -> &[Option<u32>; HOTBAR] {
        &self.guns
    }

    pub fn holds_gun(&self, inventory: &Inventory, selected: usize) -> bool {
        inventory.slots[selected].as_ref().is_some_and(|stack| {
            weapon_of(stack).is_some()
                || (is_fork_gun(stack) && self.guns.get(selected).copied().flatten().is_some())
        })
    }
}

pub struct Hotbar<'a> {
    pub selected: usize,
    pub room: usize,
    pub binding: &'a mut Binding,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Settled {
    pub arsenal: Arsenal,
    pub guns: [Option<u32>; HOTBAR],
    pub too_many: bool,
    pub dropped: Vec<ItemStack>,
}

const CLIP_KEYS: [&str; 3] = ["clip", "clip_left", "alt_clip"];

fn clips_of(stack: &ItemStack) -> [i64; 3] {
    CLIP_KEYS.map(|key| crate::gun_names::clip(stack, key).unwrap_or(0))
}

fn write_clips(stack: &mut ItemStack, clips: [i64; 3]) {
    let attachments = crate::gun_names::attachments(stack);
    let kept = [
        true,
        attachments.iter().any(|name| name == "akimbo"),
        fork_items::has_alt_mode(&attachments),
    ];
    let components = stack
        .components
        .get_or_insert_with(|| Value::Object(Map::new()));
    let Some(state) = components
        .as_object_mut()
        .map(|components| {
            components
                .entry(fork_items::CUSTOM_DATA)
                .or_insert_with(|| Value::Object(Map::new()))
        })
        .and_then(Value::as_object_mut)
        .map(|custom| {
            custom
                .entry("iw4l")
                .or_insert_with(|| Value::Object(Map::new()))
        })
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    for ((key, clip), kept) in CLIP_KEYS.iter().zip(clips).zip(kept) {
        if kept {
            state.insert((*key).to_owned(), Value::from(clip));
        }
    }
}

pub fn stock_index(id: &str) -> Option<u8> {
    fork_items::ammo_index(id).or_else(|| {
        (id == fork_items::THROWING_KNIFE)
            .then(|| u8::try_from(fork_items::AMMO_TYPES.len()).ok())
            .flatten()
    })
}

fn stock_id(index: u8) -> Option<&'static str> {
    fork_items::AMMO_TYPES
        .get(usize::from(index))
        .copied()
        .or_else(|| {
            (usize::from(index) == fork_items::AMMO_TYPES.len())
                .then_some(fork_items::THROWING_KNIFE)
        })
}

fn take_rounds(inventory: &mut Inventory, selected: usize, ammo: u8, rounds: i32) {
    let Some(id) = stock_id(ammo) else {
        return;
    };
    let mut left = rounds;
    for slot in [crate::offhand::OFFHAND, selected].into_iter().chain(0..36) {
        let Some(cell) = inventory.slots.get_mut(slot) else {
            continue;
        };
        let Some(stack) = cell.as_mut().filter(|stack| stack.id == id) else {
            continue;
        };
        if left <= 0 {
            return;
        }
        let take = left.min(i32::from(stack.count));
        stack.count -= u8::try_from(take).unwrap_or(stack.count);
        left -= take;
        if stack.count == 0 {
            *cell = None;
        }
    }
}

fn grenade_item<C: GunContent>(
    names: &mut GunNames,
    content: &C,
    weapon: u32,
) -> Option<&'static str> {
    let (id, _) = names.item(content, weapon)?;
    let row = fork_items::get(id)?;
    match row.kind {
        Kind::Lethal | Kind::Tactical => Some(row.id),
        Kind::Gun => row.ammo.filter(|&ammo| ammo == fork_items::THROWING_KNIFE),
        _ => None,
    }
}

fn holds(inventory: &Inventory, slot: usize, id: &str) -> bool {
    inventory
        .slots
        .get(slot)
        .and_then(Option::as_ref)
        .is_some_and(|stack| stack.id == id)
}

fn take_grenade(inventory: &mut Inventory, slot: usize, id: &str) {
    let from = [slot]
        .into_iter()
        .chain(0..36)
        .chain(std::iter::once(crate::offhand::OFFHAND))
        .find(|&each| holds(inventory, each, id));
    let Some(from) = from else {
        return;
    };
    let cell = &mut inventory.slots[from];
    if let Some(stack) = cell.as_mut() {
        stack.count = stack.count.saturating_sub(1);
        if stack.count == 0 {
            *cell = None;
        }
    }
}

fn refill_grenades(inventory: &mut Inventory, slot: usize, id: &str) {
    if let Some(from) = (0..36).find(|&each| holds(inventory, each, id)) {
        inventory.slots[slot] = inventory.slots[from].take();
    }
}

fn give_grenade(inventory: &mut Inventory, slot: usize, id: &str, selected: usize) -> Option<ItemStack> {
    let max = fork_items::get(id)
        .map_or(GRENADE_STACK, |row| row.max_stack)
        .clamp(1, GRENADE_STACK);
    match &mut inventory.slots[slot] {
        Some(held) if held.id == id && held.count < held.max.min(max) => {
            held.count += 1;
            None
        }
        empty @ None => {
            let mut stack = ItemStack::new(id, 1);
            stack.max = max;
            *empty = Some(stack);
            None
        }
        _ => {
            let mut stack = ItemStack::new(id, 1);
            stack.max = max;
            inventory.add_item(stack, selected)
        }
    }
}

pub fn spend_grenades(
    inventory: &mut Inventory,
    id: &str,
    delta: i32,
    selected: usize,
) -> Vec<ItemStack> {
    let Some(slot) = fork_items::grenade_slot(id) else {
        return Vec::new();
    };
    let mut dropped = Vec::new();
    if delta < 0 {
        let had = holds(inventory, slot, id);
        for _ in 0..delta.unsigned_abs() {
            take_grenade(inventory, slot, id);
        }
        if had && inventory.slots[slot].is_none() {
            refill_grenades(inventory, slot, id);
        }
    }
    for _ in 0..delta.max(0) {
        dropped.extend(give_grenade(inventory, slot, id, selected));
    }
    dropped
}

pub fn settle_arsenal<C: GunContent>(
    inventory: &mut Inventory,
    mode: &PlayerMode,
    spent: Vec<Spent>,
    (names, content): (&mut GunNames, &C),
    hotbar: Hotbar<'_>,
) -> Settled {
    let Hotbar {
        selected,
        room,
        binding,
    } = hotbar;
    let mut dropped = Vec::new();
    for spent in &spent {
        if let Spent::Grenade { weapon, delta } = *spent
            && mode.consumes_items()
            && let Some(id) = grenade_item(names, content, weapon)
        {
            dropped.extend(spend_grenades(inventory, id, delta, selected));
        }
    }
    let mut weapon_at = |inventory: &Inventory, slot: usize| {
        inventory.slots[slot]
            .as_ref()
            .filter(|stack| is_fork_gun(stack))
            .and_then(|stack| names.weapon_of(content, stack))
    };
    for spent in spent {
        if let Spent::Rounds { ammo, rounds } = spent {
            if mode.consumes_items() {
                take_rounds(inventory, selected, ammo, rounds);
            }
            continue;
        }
        let Spent::Clip {
            weapon,
            clip,
            clip_left,
            alt_clip,
        } = spent
        else {
            continue;
        };
        let bound = binding
            .bound
            .iter()
            .find(|(w, _, _)| *w == weapon)
            .map(|(_, slot, _)| *slot)
            .filter(|&slot| weapon_at(inventory, slot) == Some(weapon));
        let Some(slot) =
            bound.or_else(|| (0..HOTBAR).find(|&slot| weapon_at(inventory, slot) == Some(weapon)))
        else {
            continue;
        };
        let Some(stack) = inventory.slots[slot].as_mut() else {
            continue;
        };
        write_clips(stack, [clip, clip_left, alt_clip].map(i64::from));
        let written = clips_of(stack);
        if let Some(entry) = binding
            .bound
            .iter_mut()
            .find(|(w, s, _)| *w == weapon && *s == slot)
        {
            entry.2 = written;
        }
    }
    let mut copies: Vec<(u32, Vec<usize>)> = Vec::new();
    let mut guns = [None; HOTBAR];
    let mut inert = Vec::new();
    for (slot, gun) in guns.iter_mut().enumerate() {
        let Some(weapon) = weapon_at(inventory, slot) else {
            continue;
        };
        if let Some((_, slots)) = copies.iter_mut().find(|(w, _)| *w == weapon) {
            slots.push(slot);
        } else if copies.len() < room {
            copies.push((weapon, vec![slot]));
        } else {
            inert.push(slot);
            continue;
        }
        *gun = Some(weapon);
    }
    let mut arsenal = Arsenal {
        endless: !mode.consumes_items(),
        ..Arsenal::default()
    };
    let mut bound = Vec::new();
    for (weapon, slots) in copies {
        let slot = if slots.contains(&selected) {
            selected
        } else {
            slots[0]
        };
        if slots.contains(&selected) {
            arsenal.selected = Some(weapon);
        }
        let size = names.clip_size(content, weapon).map(i64::from);
        let Some(stack) = inventory.slots[slot].as_mut() else {
            continue;
        };
        let mut clips = clips_of(stack);
        if let Some(size) = size {
            for clip in clips.iter_mut().take(2) {
                if *clip == fork_items::FULL_CLIP {
                    *clip = size;
                }
            }
            if clips != clips_of(stack) {
                write_clips(stack, clips);
                clips = clips_of(stack);
            }
        }
        let rebind = binding
            .bound
            .iter()
            .find(|(w, _, _)| *w == weapon)
            .is_none_or(|(_, was, state)| *was != slot || *state != clips);
        let ammo = fork_items::get(&stack.id)
            .and_then(|row| row.ammo)
            .and_then(stock_index)
            .unwrap_or(NO_AMMO);
        let alt_ammo = fork_items::alt_ammo(&crate::gun_names::attachments(stack))
            .and_then(stock_index)
            .unwrap_or(NO_AMMO);
        let [clip, clip_left, alt_clip] = clips.map(|clip| i32::try_from(clip).unwrap_or(0));
        arsenal.guns.push(Gun {
            weapon,
            clip,
            clip_left,
            alt_clip,
            ammo,
            rebind,
            alt_ammo,
        });
        bound.push((weapon, slot, clips));
    }
    let counted = (0..36).chain(std::iter::once(crate::offhand::OFFHAND));
    for stack in counted.filter_map(|slot| inventory.slots.get(slot).cloned().flatten()) {
        if let Some(index) = stock_index(&stack.id) {
            arsenal.stock[usize::from(index)] += i32::from(stack.count);
        }
    }
    for (slot, published) in [
        (LETHAL_SLOT, &mut arsenal.lethal),
        (TACTICAL_SLOT, &mut arsenal.tactical),
    ] {
        *published = inventory.slots[slot]
            .as_ref()
            .filter(|stack| fork_items::grenade_slot(&stack.id) == Some(slot))
            .and_then(|stack| Some((names.weapon_of(content, stack)?, i32::from(stack.count))));
    }
    let inert_selected = inert.contains(&selected);
    let too_many = inert_selected && !binding.inert_selected;
    *binding = Binding {
        bound,
        guns,
        inert_selected,
    };
    Settled {
        arsenal,
        guns,
        too_many,
        dropped,
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

    const M4: u32 = 1;
    const USP: u32 = 2;
    const AK: u32 = 3;
    const USP_AKIMBO: u32 = 4;
    const M4_GL: u32 = 5;
    const KNIFE: u32 = 6;
    const FRAG: u32 = 7;
    const SEMTEX: u32 = 8;
    const FLASH: u32 = 9;
    const THROWING_KNIFE: u32 = 10;
    const GRENADES: [(u32, &str); 5] = [
        (FRAG, "iw4:weapon/frag_grenade_mp"),
        (SEMTEX, "iw4:weapon/semtex_mp"),
        (FLASH, "iw4:weapon/flash_grenade_mp"),
        (THROWING_KNIFE, "iw4:weapon/throwingknife_mp"),
        (KNIFE, "t5:weapon/knife_ballistic_mp"),
    ];

    struct Fake;

    impl GunContent for Fake {
        fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32> {
            let attachments: Vec<&str> = attachments.iter().map(String::as_str).collect();
            match (weapon, &attachments[..]) {
                ("iw4:weapon/m4_mp", []) => Some(M4),
                ("iw4:weapon/m4_mp", ["reflex", "gl"]) => Some(M4_GL),
                ("iw4:weapon/usp_mp", []) => Some(USP),
                ("iw4:weapon/usp_mp", ["akimbo"]) => Some(USP_AKIMBO),
                ("iw4:weapon/ak47_mp", []) => Some(AK),
                ("t5:weapon/knife_ballistic_mp", []) => Some(KNIFE),
                (key, []) => GRENADES
                    .iter()
                    .find(|(_, each)| *each == key)
                    .map(|(index, _)| *index),
                _ => None,
            }
        }

        fn selection(&self, index: u32) -> Option<(String, Vec<String>)> {
            GRENADES
                .iter()
                .find(|(each, _)| *each == index)
                .map(|(_, key)| (key.to_string(), Vec::new()))
        }

        fn clip_size(&self, index: u32) -> Option<i32> {
            match index {
                M4 | AK | M4_GL => Some(30),
                USP | USP_AKIMBO => Some(12),
                KNIFE => Some(1),
                _ => None,
            }
        }
    }

    fn fork_gun(id: &str, state: Value) -> ItemStack {
        let mut stack = ItemStack::new(id, 1);
        stack.max = 1;
        stack.components = Some(serde_json::json!({ fork_items::CUSTOM_DATA: { "iw4l": state } }));
        stack
    }

    fn settle(
        inventory: &mut Inventory,
        names: &mut GunNames,
        binding: &mut Binding,
        (selected, room): (usize, usize),
        spent: Vec<Spent>,
    ) -> Settled {
        let hotbar = Hotbar {
            selected,
            room,
            binding,
        };
        settle_arsenal(
            inventory,
            &PlayerMode::default(),
            spent,
            (names, &Fake),
            hotbar,
        )
    }

    fn weapons(settled: &Settled) -> Vec<u32> {
        settled.arsenal.guns.iter().map(|gun| gun.weapon).collect()
    }

    fn clip(inventory: &Inventory, slot: usize, key: &str) -> Option<i64> {
        inventory.slots[slot]
            .as_ref()
            .and_then(|stack| crate::gun_names::clip(stack, key))
    }

    #[test]
    fn a_gun_moved_into_a_chest_is_not_given_again() {
        use crate::menus::{Books, Open};
        use minecraft_net::messages::{MenuAction, MenuSlot};
        use minecraft_terrain::containers::{Containers, Opened, Station};
        let mut chat = minecraft_commands::CommandServer::new(crate::commands::HOST);
        let id = minecraft_commands::PlayerId(0);
        chat.join(id, "Steve");
        let mut player = crate::player::Player::new(
            chat.profile(id).unwrap(),
            PlayerMode::default(),
            crate::survival::Survival::default(),
            Inventory::default(),
        );
        player.inventory.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 30 })));
        player.inventory.slots[1] = Some(ItemStack::new("minecraft:dirt", 4));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let before = settle(
            &mut player.inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        assert_eq!(weapons(&before), vec![M4]);
        let chest = Opened {
            station: Station::Chest,
            cells: vec![(4, 64, 4)],
        };
        let title = minecraft_commands::Text::literal("Chest");
        assert!(crate::menus::open(
            &mut player,
            Open::Block(chest.clone()),
            title,
            0
        ));
        let mut containers = Containers::default();
        let stations = minecraft_terrain::stations::StationBook::default();
        let books = Books {
            stations: &stations,
            enchantments: None,
            guns: None,
        };
        let shift = MenuAction::Slot {
            slot: MenuSlot::Inventory(0),
            right: false,
            shift: true,
        };
        crate::menus::apply(&mut player, &mut containers, books, &shift, 0.5);
        let after = settle(
            &mut player.inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        assert!(after.arsenal.guns.is_empty(), "MW2 no longer holds the M4");
        assert_eq!(after.guns, [None; HOTBAR]);
        let in_chest = containers.cells(&chest);
        let everywhere = player
            .inventory
            .slots
            .iter()
            .chain(std::iter::once(&player.inventory.cursor))
            .chain(in_chest.iter())
            .flatten()
            .filter(|stack| is_fork_gun(stack))
            .count();
        assert_eq!(everywhere, 1, "one gun in all, and it is in the chest");
        assert!(in_chest.iter().flatten().any(|stack| stack.id == "iw4l:m4"));
    }

    #[test]
    fn only_hotbar_guns_reach_mw2_once_each() {
        let mut inventory = Inventory::default();
        let loaded = serde_json::json!({ "clip": 9 });
        inventory.slots[0] = Some(fork_gun("iw4l:m4", loaded.clone()));
        inventory.slots[2] = Some(fork_gun("iw4l:usp", loaded.clone()));
        inventory.slots[3] = Some(weapon_item(77));
        inventory.slots[4] = Some(fork_gun("iw4l:m4", loaded.clone()));
        inventory.slots[12] = Some(fork_gun("iw4l:ak47", loaded));
        inventory.slots[13] = Some(ItemStack::new(fork_items::RIFLE_AMMO, 40));
        inventory.slots[30] = Some(ItemStack::new(fork_items::RIFLE_AMMO, 64));
        inventory.slots[crate::offhand::OFFHAND] = Some(ItemStack::new(fork_items::PISTOL_AMMO, 5));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (2, 13),
            Vec::new(),
        );
        assert_eq!(
            weapons(&settled),
            vec![M4, USP],
            "hotbar order, each gun once"
        );
        assert_eq!(settled.arsenal.selected, Some(USP));
        let mut guns = [None; HOTBAR];
        guns[0] = Some(M4);
        guns[2] = Some(USP);
        guns[4] = Some(M4);
        assert_eq!(settled.guns, guns, "both copies raise the M4");
        let rifle = fork_items::ammo_index(fork_items::RIFLE_AMMO).unwrap();
        let pistol = fork_items::ammo_index(fork_items::PISTOL_AMMO).unwrap();
        assert_eq!(settled.arsenal.guns[0].ammo, rifle);
        assert_eq!(settled.arsenal.stock[usize::from(rifle)], 104);
        assert_eq!(settled.arsenal.stock[usize::from(pistol)], 5);
        assert!(!settled.arsenal.endless);
        assert!(binding.holds_gun(&inventory, 4));
        assert!(
            binding.holds_gun(&inventory, 3),
            "a killstreak item is a gun"
        );
        assert!(!binding.holds_gun(&inventory, 1));
        let creative = PlayerMode::new(minecraftoss_player::GameMode::Creative);
        let hotbar = Hotbar {
            selected: 0,
            room: 13,
            binding: &mut binding,
        };
        let endless = settle_arsenal(
            &mut inventory,
            &creative,
            Vec::new(),
            (&mut names, &Fake),
            hotbar,
        );
        assert!(endless.arsenal.endless);
    }

    #[test]
    fn a_gun_past_the_free_weapon_slots_is_inert() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({})));
        inventory.slots[1] = Some(fork_gun("iw4l:usp", serde_json::json!({})));
        inventory.slots[2] = Some(fork_gun("iw4l:m4", serde_json::json!({})));
        inventory.slots[3] = Some(fork_gun("iw4l:ak47", serde_json::json!({})));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let settled = settle(&mut inventory, &mut names, &mut binding, (0, 2), Vec::new());
        assert_eq!(weapons(&settled), vec![M4, USP]);
        assert_eq!(settled.guns[2], Some(M4), "a copy takes no weapon slot");
        assert_eq!(settled.guns[3], None);
        assert!(!settled.too_many);
        let settled = settle(&mut inventory, &mut names, &mut binding, (3, 2), Vec::new());
        assert!(settled.too_many, "selecting the inert gun says so once");
        assert!(!binding.holds_gun(&inventory, 3));
        assert_eq!(settled.arsenal.selected, None);
        let settled = settle(&mut inventory, &mut names, &mut binding, (3, 2), Vec::new());
        assert!(!settled.too_many);
        let settled = settle(&mut inventory, &mut names, &mut binding, (3, 3), Vec::new());
        assert_eq!(weapons(&settled), vec![M4, USP, AK]);
        assert!(binding.holds_gun(&inventory, 3));
    }

    #[test]
    fn two_copies_bind_the_selected_one() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 10 })));
        inventory.slots[3] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 25 })));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let first = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        assert_eq!(first.arsenal.guns[0].clip, 10);
        assert!(first.arsenal.guns[0].rebind, "a gun MW2 did not hold yet");
        let again = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        assert!(!again.arsenal.guns[0].rebind);
        let other = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (5, 13),
            Vec::new(),
        );
        assert_eq!(
            other.arsenal.guns[0].clip, 10,
            "the first copy when neither is selected"
        );
        assert!(!other.arsenal.guns[0].rebind);
        let switched = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (3, 13),
            Vec::new(),
        );
        assert_eq!(switched.arsenal.guns[0].clip, 25);
        assert!(switched.arsenal.guns[0].rebind);
        let fired = Spent::Clip {
            weapon: M4,
            clip: 20,
            clip_left: 0,
            alt_clip: 0,
        };
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (3, 13),
            vec![fired],
        );
        assert_eq!(clip(&inventory, 3, "clip"), Some(20));
        assert_eq!(
            clip(&inventory, 0, "clip"),
            Some(10),
            "the other copy keeps its clip"
        );
        assert_eq!(settled.arsenal.guns[0].clip, 20);
        assert!(!settled.arsenal.guns[0].rebind);
        inventory.slots.swap(3, 20);
        inventory.slots[3] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 30 })));
        let swapped = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (3, 13),
            Vec::new(),
        );
        assert!(
            swapped.arsenal.guns[0].rebind,
            "another M4 in the same slot loads its own clip"
        );
    }

    #[test]
    fn a_clip_spent_is_written_to_the_bound_item() {
        let mut inventory = Inventory::default();
        let state =
            serde_json::json!({ "attachments": ["reflex", "gl"], "clip": -1, "alt_clip": -1 });
        inventory.slots[1] = Some(fork_gun("iw4l:m4", state));
        let akimbo = serde_json::json!({ "attachments": ["akimbo"], "clip": -1, "clip_left": -1 });
        inventory.slots[2] = Some(fork_gun("iw4l:usp", akimbo));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (1, 13),
            Vec::new(),
        );
        assert_eq!(weapons(&settled), vec![M4_GL, USP_AKIMBO]);
        assert_eq!(
            clip(&inventory, 1, "clip"),
            Some(30),
            "a full clip gets its size when bound"
        );
        assert_eq!(clip(&inventory, 1, "alt_clip"), Some(-1));
        assert_eq!(clip(&inventory, 2, "clip_left"), Some(12));
        assert_eq!(settled.arsenal.guns[0].alt_clip, -1);
        let spent = vec![
            Spent::Clip {
                weapon: M4_GL,
                clip: 12,
                clip_left: 0,
                alt_clip: 1,
            },
            Spent::Clip {
                weapon: USP_AKIMBO,
                clip: 3,
                clip_left: 7,
                alt_clip: 0,
            },
        ];
        let settled = settle(&mut inventory, &mut names, &mut binding, (1, 13), spent);
        assert_eq!(clip(&inventory, 1, "clip"), Some(12));
        assert_eq!(clip(&inventory, 1, "alt_clip"), Some(1));
        assert_eq!(clip(&inventory, 1, "clip_left"), None, "not akimbo");
        assert_eq!(clip(&inventory, 2, "clip"), Some(3));
        assert_eq!(clip(&inventory, 2, "clip_left"), Some(7));
        assert_eq!(clip(&inventory, 2, "alt_clip"), None);
        assert_eq!(
            settled.arsenal.guns[0],
            Gun {
                weapon: M4_GL,
                clip: 12,
                clip_left: 0,
                alt_clip: 1,
                ammo: fork_items::ammo_index(fork_items::RIFLE_AMMO).unwrap(),
                rebind: false,
                alt_ammo: fork_items::ammo_index(fork_items::EXPLOSIVE_ROUND).unwrap(),
            }
        );
        assert_eq!(
            crate::gun_names::attachments(inventory.slots[1].as_ref().unwrap()),
            vec!["reflex".to_owned(), "gl".to_owned()]
        );
    }

    fn count(inventory: &Inventory, slot: usize) -> u8 {
        inventory.slots[slot]
            .as_ref()
            .map_or(0, |stack| stack.count)
    }

    fn rifle_rounds(rounds: i32) -> Spent {
        Spent::Rounds {
            ammo: stock_index(fork_items::RIFLE_AMMO).unwrap(),
            rounds,
        }
    }

    #[test]
    fn a_reload_takes_exactly_the_missing_rounds_offhand_first() {
        let rifle = |count| Some(ItemStack::new(fork_items::RIFLE_AMMO, count));
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 12 })));
        inventory.slots[1] = rifle(7);
        inventory.slots[2] = rifle(3);
        inventory.slots[5] = rifle(64);
        inventory.slots[20] = Some(ItemStack::new(fork_items::PISTOL_AMMO, 30));
        inventory.slots[crate::offhand::OFFHAND] = rifle(10);
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let index = usize::from(stock_index(fork_items::RIFLE_AMMO).unwrap());
        let before = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (2, 13),
            Vec::new(),
        );
        assert_eq!(before.arsenal.stock[index], 84);
        let after = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (2, 13),
            vec![rifle_rounds(18)],
        );
        assert_eq!(
            count(&inventory, crate::offhand::OFFHAND),
            0,
            "offhand first"
        );
        assert_eq!(count(&inventory, 2), 0, "then the main hand");
        assert_eq!(count(&inventory, 1), 2, "then slots 0-35 in order");
        assert_eq!(count(&inventory, 5), 64);
        assert_eq!(count(&inventory, 20), 30, "other ammo types stay");
        assert_eq!(after.arsenal.stock[index], 66);
        let partial = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (2, 13),
            vec![rifle_rounds(3)],
        );
        assert_eq!((count(&inventory, 1), count(&inventory, 5)), (0, 63));
        assert_eq!(partial.arsenal.stock[index], 63);
        let short = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (2, 13),
            vec![rifle_rounds(100)],
        );
        assert_eq!(short.arsenal.stock[index], 0, "never below nothing");
        assert_eq!(count(&inventory, 20), 30);
        assert!(is_fork_gun(inventory.slots[0].as_ref().unwrap()));
    }

    #[test]
    fn creative_takes_no_rounds_and_has_endless_stock() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 0 })));
        inventory.slots[5] = Some(ItemStack::new(fork_items::RIFLE_AMMO, 20));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let hotbar = Hotbar {
            selected: 0,
            room: 13,
            binding: &mut binding,
        };
        let creative = PlayerMode::new(minecraftoss_player::GameMode::Creative);
        let settled = settle_arsenal(
            &mut inventory,
            &creative,
            vec![rifle_rounds(18)],
            (&mut names, &Fake),
            hotbar,
        );
        assert!(settled.arsenal.endless);
        assert_eq!(count(&inventory, 5), 20);
    }

    #[test]
    fn the_ballistic_knife_reloads_from_throwing_knives() {
        let mut inventory = Inventory::default();
        let knife = fork_gun("iw4l:t5_knife_ballistic", serde_json::json!({ "clip": 0 }));
        inventory.slots[0] = Some(knife);
        inventory.slots[10] = Some(ItemStack::new(fork_items::THROWING_KNIFE, 5));
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        let index = stock_index(fork_items::THROWING_KNIFE).unwrap();
        assert_eq!(usize::from(index), AMMO_TYPES - 1);
        assert_eq!(weapons(&settled), vec![KNIFE]);
        assert_eq!(settled.arsenal.guns[0].ammo, index);
        assert_eq!(settled.arsenal.stock[usize::from(index)], 5);
        let spent = vec![Spent::Rounds {
            ammo: index,
            rounds: 1,
        }];
        let settled = settle(&mut inventory, &mut names, &mut binding, (0, 13), spent);
        assert_eq!(count(&inventory, 10), 4);
        assert_eq!(settled.arsenal.stock[usize::from(index)], 4);
    }

    #[test]
    fn each_player_s_arsenal_counts_their_own_inventory() {
        let mut names = GunNames::default();
        let mut host = Inventory::default();
        host.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 30 })));
        host.slots[9] = Some(ItemStack::new(fork_items::RIFLE_AMMO, 64));
        let mut friend = Inventory::default();
        friend.slots[0] = Some(fork_gun("iw4l:m4", serde_json::json!({ "clip": 2 })));
        friend.slots[9] = Some(ItemStack::new(fork_items::RIFLE_AMMO, 40));
        let (mut host_binding, mut friend_binding) = (Binding::default(), Binding::default());
        let index = usize::from(stock_index(fork_items::RIFLE_AMMO).unwrap());
        let host_settled = settle(
            &mut host,
            &mut names,
            &mut host_binding,
            (0, 13),
            Vec::new(),
        );
        let friend_settled = settle(
            &mut friend,
            &mut names,
            &mut friend_binding,
            (0, 13),
            vec![rifle_rounds(28)],
        );
        assert_eq!(host_settled.arsenal.stock[index], 64);
        assert_eq!(friend_settled.arsenal.stock[index], 12);
        assert_eq!(friend_settled.arsenal.guns[0].clip, 2);
        assert_eq!((count(&host, 9), count(&friend, 9)), (64, 12));
    }

    fn grenade(id: &str, count: u8) -> Option<ItemStack> {
        let mut stack = ItemStack::new(id, count);
        stack.max = GRENADE_STACK;
        Some(stack)
    }

    fn thrown(weapon: u32, delta: i32) -> Spent {
        Spent::Grenade { weapon, delta }
    }

    #[test]
    fn a_throw_takes_one_from_its_slot() {
        let mut inventory = Inventory::default();
        inventory.slots[LETHAL_SLOT] = grenade("iw4l:frag_grenade", 3);
        inventory.slots[TACTICAL_SLOT] = grenade("iw4l:flash_grenade", 2);
        inventory.slots[12] = grenade("iw4l:frag_grenade", 10);
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let before = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            Vec::new(),
        );
        assert_eq!(before.arsenal.lethal, Some((FRAG, 3)));
        assert_eq!(before.arsenal.tactical, Some((FLASH, 2)));
        let after = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            vec![thrown(FRAG, -1), thrown(FLASH, -1)],
        );
        assert_eq!(count(&inventory, LETHAL_SLOT), 2);
        assert_eq!(count(&inventory, TACTICAL_SLOT), 1);
        assert_eq!(count(&inventory, 12), 10, "the backpack stays");
        assert_eq!(after.arsenal.lethal, Some((FRAG, 2)));
        assert_eq!(after.arsenal.tactical, Some((FLASH, 1)));
        assert!(after.dropped.is_empty());
        inventory.slots[LETHAL_SLOT] = grenade("iw4l:semtex", 4);
        settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            vec![thrown(FRAG, -1)],
        );
        assert_eq!(
            (count(&inventory, LETHAL_SLOT), count(&inventory, 12)),
            (4, 9),
            "a frag thrown before the slot changed comes out of the backpack"
        );
    }

    #[test]
    fn an_emptied_slot_refills_with_the_same_grenade_only() {
        let mut inventory = Inventory::default();
        inventory.slots[LETHAL_SLOT] = grenade("iw4l:frag_grenade", 1);
        inventory.slots[10] = grenade("iw4l:semtex", 16);
        inventory.slots[20] = grenade("iw4l:frag_grenade", 5);
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            vec![thrown(FRAG, -1)],
        );
        assert_eq!(count(&inventory, LETHAL_SLOT), 5);
        assert!(inventory.slots[20].is_none());
        assert_eq!(settled.arsenal.lethal, Some((FRAG, 5)));
        inventory.slots[LETHAL_SLOT] = grenade("iw4l:frag_grenade", 1);
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            vec![thrown(FRAG, -1)],
        );
        assert!(inventory.slots[LETHAL_SLOT].is_none(), "never semtex");
        assert_eq!(count(&inventory, 10), 16);
        assert_eq!(settled.arsenal.lethal, None);
    }

    #[test]
    fn creative_keeps_the_stack() {
        let mut inventory = Inventory::default();
        inventory.slots[LETHAL_SLOT] = grenade("iw4l:frag_grenade", 3);
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let hotbar = Hotbar {
            selected: 0,
            room: 13,
            binding: &mut binding,
        };
        let creative = PlayerMode::new(minecraftoss_player::GameMode::Creative);
        let settled = settle_arsenal(
            &mut inventory,
            &creative,
            vec![thrown(FRAG, -1)],
            (&mut names, &Fake),
            hotbar,
        );
        assert_eq!(count(&inventory, LETHAL_SLOT), 3);
        assert_eq!(settled.arsenal.lethal, Some((FRAG, 3)));
    }

    #[test]
    fn a_picked_up_knife_comes_back_and_overflows_like_a_pickup() {
        let mut inventory = Inventory::default();
        inventory.slots[LETHAL_SLOT] = grenade(fork_items::THROWING_KNIFE, 63);
        let (mut names, mut binding) = (GunNames::default(), Binding::default());
        let picked = vec![thrown(THROWING_KNIFE, 1), thrown(KNIFE, 1)];
        let settled = settle(&mut inventory, &mut names, &mut binding, (0, 13), picked);
        assert_eq!(count(&inventory, LETHAL_SLOT), 64);
        let elsewhere: u8 = inventory.slots[..36]
            .iter()
            .flatten()
            .filter(|stack| stack.id == fork_items::THROWING_KNIFE)
            .map(|stack| stack.count)
            .sum();
        assert_eq!(elsewhere, 1, "a full slot sends the knife on");
        assert!(settled.dropped.is_empty());
        for slot in 0..36 {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:dirt", 64));
        }
        let settled = settle(
            &mut inventory,
            &mut names,
            &mut binding,
            (0, 13),
            vec![thrown(THROWING_KNIFE, 1)],
        );
        let dropped: Vec<(&str, u8)> = settled
            .dropped
            .iter()
            .map(|stack| (stack.id.as_str(), stack.count))
            .collect();
        assert_eq!(dropped, vec![(fork_items::THROWING_KNIFE, 1)]);
    }
}
