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
pub const AMMO_TYPES: usize = fork_items::AMMO_TYPES.len();
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Gun {
    pub weapon: u32,
    pub clip: i32,
    pub clip_left: i32,
    pub alt_clip: i32,
    pub ammo: u8,
    pub rebind: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arsenal {
    pub guns: Vec<Gun>,
    pub selected: Option<u32>,
    pub stock: [i32; AMMO_TYPES],
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
    let mut weapon_at = |inventory: &Inventory, slot: usize| {
        inventory.slots[slot]
            .as_ref()
            .filter(|stack| is_fork_gun(stack))
            .and_then(|stack| names.weapon_of(content, stack))
    };
    for spent in spent {
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
    for slot in 0..HOTBAR {
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
        guns[slot] = Some(weapon);
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
            .and_then(fork_items::ammo_index)
            .unwrap_or(NO_AMMO);
        let [clip, clip_left, alt_clip] = clips.map(|clip| i32::try_from(clip).unwrap_or(0));
        arsenal.guns.push(Gun {
            weapon,
            clip,
            clip_left,
            alt_clip,
            ammo,
            rebind,
        });
        bound.push((weapon, slot, clips));
    }
    let counted = (0..36).chain(std::iter::once(crate::offhand::OFFHAND));
    for stack in counted.filter_map(|slot| inventory.slots.get(slot).cloned().flatten()) {
        if let Some(index) = fork_items::ammo_index(&stack.id) {
            arsenal.stock[usize::from(index)] += i32::from(stack.count);
        }
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
                _ => None,
            }
        }

        fn selection(&self, _: u32) -> Option<(String, Vec<String>)> {
            None
        }

        fn clip_size(&self, index: u32) -> Option<i32> {
            match index {
                M4 | AK | M4_GL => Some(30),
                USP | USP_AKIMBO => Some(12),
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
            }
        );
        assert_eq!(
            crate::gun_names::attachments(inventory.slots[1].as_ref().unwrap()),
            vec!["reflex".to_owned(), "gl".to_owned()]
        );
    }
}
