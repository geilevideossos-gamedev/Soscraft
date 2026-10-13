use minecraft_terrain::fork_items::{self, Bucket, Kind};
use minecraftoss_player::inventory::{Inventory, ItemStack};
use serde_json::json;

use crate::gun_names::{GunContent, GunNames};

pub const GIVEN_TAG: &str = "iw4l_loadout_given";
pub const MAGAZINES: u32 = 3;
const BACKPACK_FIRST: [std::ops::Range<usize>; 2] = [9..36, 0..9];
const SLOT_STAND_INS: [&str; 2] = ["iw4l:m4", "iw4l:usp"];
pub const CLASS_GRENADES: u32 = 32;
const GRENADE_STAND_INS: [(Kind, usize, &str); 2] = [
    (Kind::Lethal, fork_items::LETHAL_SLOT, "iw4l:frag_grenade"),
    (Kind::Tactical, fork_items::TACTICAL_SLOT, "iw4l:flash_grenade"),
];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Class {
    pub primary: u32,
    pub secondary: u32,
    pub lethal: u32,
    pub tactical: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Given {
    pub dropped: Vec<ItemStack>,
    pub stood_in: Vec<String>,
}

pub fn stand_in(bucket: Bucket) -> Option<&'static str> {
    Some(match bucket {
        Bucket::AssaultRifle => "iw4l:m4",
        Bucket::Smg => "iw4l:mp5k",
        Bucket::Lmg => "iw4l:rpd",
        Bucket::Sniper => "iw4l:m21",
        Bucket::Shotgun => "iw4l:m1014",
        Bucket::MachinePistol => "iw4l:glock",
        Bucket::Pistol => "iw4l:usp",
        Bucket::Launcher => "iw4l:at4",
        _ => return None,
    })
}

struct ClassGun {
    id: &'static str,
    attachments: Vec<String>,
    weapon: Option<u32>,
}

fn class_gun<C: GunContent>(
    (names, content): (&mut GunNames, &C),
    weapon: u32,
    slot: usize,
    stood_in: &mut Vec<String>,
) -> Option<ClassGun> {
    if weapon == 0 {
        return None;
    }
    let found = names
        .item(content, weapon)
        .filter(|(id, _)| fork_items::get(id).is_some_and(|row| row.kind == Kind::Gun));
    if let Some((id, attachments)) = found {
        return Some(ClassGun {
            id,
            attachments,
            weapon: Some(weapon),
        });
    }
    let name = content
        .selection(weapon)
        .map_or_else(|| format!("weapon {weapon}"), |(key, _)| key);
    stood_in.push(name);
    let id = content
        .bucket(weapon)
        .and_then(stand_in)
        .unwrap_or(SLOT_STAND_INS[slot]);
    Some(ClassGun {
        id,
        attachments: Vec::new(),
        weapon: names.weapon(content, id, &[]),
    })
}

fn gun_stack(gun: &ClassGun) -> ItemStack {
    let mut stack = ItemStack::new(gun.id, 1);
    stack.max = 1;
    let components = (!gun.attachments.is_empty())
        .then(|| json!({"minecraft:custom_data": {"iw4l": {"attachments": gun.attachments}}}));
    stack.components = fork_items::with_full_clip(gun.id, components);
    stack
}

fn stow(inventory: &mut Inventory, id: &str, count: u32) -> Vec<ItemStack> {
    let max = fork_items::get(id).map_or(64, |row| row.max_stack).max(1);
    let mut left = count;
    let slots = || BACKPACK_FIRST.into_iter().flatten();
    for slot in slots() {
        let Some(held) = inventory.slots[slot]
            .as_mut()
            .filter(|held| held.id == id && held.components.is_none())
        else {
            continue;
        };
        let moved = u32::from(max.saturating_sub(held.count)).min(left);
        held.count += moved as u8;
        left -= moved;
    }
    let mut stacks = Vec::new();
    while left > 0 {
        let size = left.min(u32::from(max));
        left -= size;
        let mut stack = ItemStack::new(id, size as u8);
        stack.max = max;
        stacks.push(stack);
    }
    let mut dropped = Vec::new();
    for stack in stacks {
        match slots().find(|&slot| inventory.slots[slot].is_none()) {
            Some(slot) => inventory.slots[slot] = Some(stack),
            None => dropped.push(stack),
        }
    }
    dropped
}

fn class_grenade<C: GunContent>(
    (names, content): (&mut GunNames, &C),
    weapon: u32,
    kind: Kind,
) -> Option<&'static str> {
    if weapon == 0 {
        return None;
    }
    let (id, _) = names.item(content, weapon)?;
    fork_items::get(id)
        .filter(|row| row.kind == kind)
        .map(|row| row.id)
}

fn give_grenades(inventory: &mut Inventory, slot: usize, id: &str, count: u32) -> Vec<ItemStack> {
    let max = fork_items::get(id).map_or(64, |row| row.max_stack).max(1);
    let mut left = count;
    match &mut inventory.slots[slot] {
        Some(held) if held.id == id && held.components.is_none() => {
            let moved = u32::from(max.saturating_sub(held.count)).min(left);
            held.count += moved as u8;
            left -= moved;
        }
        empty @ None => {
            let size = left.min(u32::from(max));
            let mut stack = ItemStack::new(id, size as u8);
            stack.max = max;
            *empty = Some(stack);
            left -= size;
        }
        Some(_) => {}
    }
    stow(inventory, id, left)
}

pub fn give_first_loadout<C: GunContent>(
    (inventory, selected, tags): (&mut Inventory, usize, &mut Vec<String>),
    class: Option<&Class>,
    guns: (&mut GunNames, &C),
) -> Option<Given> {
    let class = class.filter(|class| class.primary != 0 || class.secondary != 0)?;
    if !minecraft_commands::tag::add(tags, GIVEN_TAG) {
        return None;
    }
    let (names, content) = guns;
    let mut given = Given::default();
    let mut rounds: Vec<(&'static str, u32)> = Vec::new();
    for (slot, weapon) in [class.primary, class.secondary].into_iter().enumerate() {
        let Some(gun) = class_gun((&mut *names, content), weapon, slot, &mut given.stood_in) else {
            continue;
        };
        let stack = gun_stack(&gun);
        if inventory.slots[slot].is_none() {
            inventory.slots[slot] = Some(stack);
        } else if let Some(rest) = inventory.add_item(stack, selected) {
            given.dropped.push(rest);
        }
        let ammo = fork_items::get(gun.id).and_then(|row| row.ammo);
        let size = gun
            .weapon
            .and_then(|weapon| names.clip_size(content, weapon));
        if let (Some(ammo), Some(size)) = (ammo, size.and_then(|size| u32::try_from(size).ok())) {
            match rounds.iter_mut().find(|(each, _)| *each == ammo) {
                Some((_, count)) => *count += size * MAGAZINES,
                None => rounds.push((ammo, size * MAGAZINES)),
            }
        }
    }
    for (ammo, count) in rounds {
        given.dropped.extend(stow(inventory, ammo, count));
    }
    for ((kind, slot, stand_in), weapon) in GRENADE_STAND_INS
        .into_iter()
        .zip([class.lethal, class.tactical])
    {
        let id = class_grenade((&mut *names, content), weapon, kind).unwrap_or(stand_in);
        given
            .dropped
            .extend(give_grenades(inventory, slot, id, CLASS_GRENADES));
    }
    Some(given)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gun_names::{attachments, clip};

    type Row = (&'static str, &'static [&'static str], i32, Option<Bucket>);

    const WEAPONS: [Row; 11] = [
        ("iw4:weapon/m4_mp", &[], 30, Some(Bucket::AssaultRifle)),
        (
            "iw4:weapon/m4_mp",
            &["reflex", "silencer"],
            30,
            Some(Bucket::AssaultRifle),
        ),
        ("iw4:weapon/usp_mp", &[], 12, Some(Bucket::Pistol)),
        ("iw4:weapon/usp_mp", &["akimbo"], 12, Some(Bucket::Pistol)),
        ("iw4:weapon/mp5k_mp", &[], 30, Some(Bucket::Smg)),
        ("iw4:weapon/new_shotgun_mp", &[], 6, Some(Bucket::Shotgun)),
        ("t5:weapon/strange_mp", &[], 10, None),
        ("iw4:weapon/m1014_mp", &[], 4, Some(Bucket::Shotgun)),
        ("iw4:weapon/cheytac_mp", &[], 5, Some(Bucket::Sniper)),
        ("iw4:weapon/semtex_mp", &[], 1, None),
        ("iw4:weapon/concussion_grenade_mp", &[], 1, None),
    ];

    struct Fake;

    fn row(index: u32) -> Option<&'static Row> {
        WEAPONS.get(index.checked_sub(1)? as usize)
    }

    impl GunContent for Fake {
        fn configuration(&self, weapon: &str, attachments: &[String]) -> Option<u32> {
            WEAPONS
                .iter()
                .position(|(key, with, _, _)| *key == weapon && *with == attachments)
                .map(|index| index as u32 + 1)
        }

        fn selection(&self, index: u32) -> Option<(String, Vec<String>)> {
            let (key, with, _, _) = row(index)?;
            Some((
                key.to_string(),
                with.iter().map(|s| s.to_string()).collect(),
            ))
        }

        fn clip_size(&self, index: u32) -> Option<i32> {
            row(index).map(|weapon| weapon.2)
        }

        fn bucket(&self, index: u32) -> Option<Bucket> {
            row(index)?.3
        }
    }

    fn index(key: &str, with: &[&str]) -> u32 {
        let with: Vec<String> = with.iter().map(|s| s.to_string()).collect();
        Fake.configuration(key, &with).unwrap()
    }

    fn class(primary: u32, secondary: u32) -> Class {
        Class {
            primary,
            secondary,
            ..Class::default()
        }
    }

    fn give(
        inventory: &mut Inventory,
        tags: &mut Vec<String>,
        class: Option<&Class>,
        names: &mut GunNames,
    ) -> Option<Given> {
        give_first_loadout((inventory, 0, tags), class, (names, &Fake))
    }

    fn count(inventory: &Inventory, id: &str) -> u32 {
        inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.id == id)
            .map(|stack| u32::from(stack.count))
            .sum()
    }

    #[test]
    fn the_class_guns_and_three_magazines_are_given_once_and_the_tag_is_set() {
        let mut inventory = Inventory::default();
        let mut tags = vec!["builder".to_owned()];
        let mut names = GunNames::default();
        let m4 = index("iw4:weapon/m4_mp", &[]);
        let usp = index("iw4:weapon/usp_mp", &[]);
        let picked = class(m4, usp);
        let given = give(&mut inventory, &mut tags, Some(&picked), &mut names).unwrap();
        assert!(given.dropped.is_empty() && given.stood_in.is_empty());
        assert_eq!(tags, vec!["builder".to_owned(), GIVEN_TAG.to_owned()]);
        let primary = inventory.slots[0].clone().unwrap();
        let secondary = inventory.slots[1].clone().unwrap();
        assert_eq!((primary.id.as_str(), primary.max), ("iw4l:m4", 1));
        assert_eq!(secondary.id, "iw4l:usp");
        assert_eq!(clip(&primary, "clip"), Some(fork_items::FULL_CLIP));
        assert_eq!(count(&inventory, fork_items::RIFLE_AMMO), 90);
        assert_eq!(count(&inventory, fork_items::PISTOL_AMMO), 36);
        let backpack: Vec<(&str, u8)> = inventory.slots[9..12]
            .iter()
            .flatten()
            .map(|stack| (stack.id.as_str(), stack.count))
            .collect();
        let (rifle, pistol) = (fork_items::RIFLE_AMMO, fork_items::PISTOL_AMMO);
        assert_eq!(backpack, [(rifle, 64), (rifle, 26), (pistol, 36)]);
        assert!(inventory.slots[2..9].iter().all(Option::is_none));
        let before = inventory.slots.clone();
        assert!(give(&mut inventory, &mut tags, Some(&picked), &mut names).is_none());
        assert_eq!(inventory.slots, before);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn no_class_or_an_empty_class_gives_nothing_and_sets_no_tag() {
        let mut inventory = Inventory::default();
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        assert!(give(&mut inventory, &mut tags, None, &mut names).is_none());
        assert!(give(&mut inventory, &mut tags, Some(&class(0, 0)), &mut names).is_none());
        assert!(tags.is_empty());
        assert!(inventory.slots.iter().all(Option::is_none));
    }

    #[test]
    fn class_attachments_become_item_attachments_and_akimbo_fills_both_clips() {
        let mut inventory = Inventory::default();
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        let m4 = index("iw4:weapon/m4_mp", &["reflex", "silencer"]);
        let usp = index("iw4:weapon/usp_mp", &["akimbo"]);
        give(&mut inventory, &mut tags, Some(&class(m4, usp)), &mut names).unwrap();
        let primary = inventory.slots[0].clone().unwrap();
        assert_eq!(attachments(&primary), vec!["reflex", "silencer"]);
        assert_eq!(clip(&primary, "clip"), Some(fork_items::FULL_CLIP));
        assert_eq!(clip(&primary, "clip_left"), None);
        let secondary = inventory.slots[1].clone().unwrap();
        assert_eq!(attachments(&secondary), vec!["akimbo"]);
        assert_eq!(clip(&secondary, "clip"), Some(fork_items::FULL_CLIP));
        assert_eq!(clip(&secondary, "clip_left"), Some(fork_items::FULL_CLIP));
        assert_eq!(count(&inventory, fork_items::PISTOL_AMMO), 36);
    }

    #[test]
    fn shared_ammo_adds_up_and_a_taken_slot_uses_the_vanilla_add_order() {
        let mut inventory = Inventory::default();
        inventory.slots[0] = Some(ItemStack::new("minecraft:dirt", 64));
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        let mp5k = index("iw4:weapon/mp5k_mp", &[]);
        let usp = index("iw4:weapon/usp_mp", &[]);
        give(
            &mut inventory,
            &mut tags,
            Some(&class(mp5k, usp)),
            &mut names,
        )
        .unwrap();
        let id = |slot: usize| inventory.slots[slot].as_ref().map(|stack| stack.id.clone());
        assert_eq!(id(0).as_deref(), Some("minecraft:dirt"));
        assert_eq!(id(1).as_deref(), Some("iw4l:mp5k"));
        assert_eq!(id(2).as_deref(), Some("iw4l:usp"));
        assert_eq!(count(&inventory, fork_items::PISTOL_AMMO), 3 * 30 + 3 * 12);
        let stacks: Vec<u8> = inventory
            .slots
            .iter()
            .flatten()
            .filter(|stack| stack.id == fork_items::PISTOL_AMMO)
            .map(|stack| stack.count)
            .collect();
        assert_eq!(stacks, vec![64, 62]);
    }

    #[test]
    fn what_does_not_fit_is_dropped() {
        let mut inventory = Inventory::default();
        for slot in 0..36 {
            inventory.slots[slot] = Some(ItemStack::new("minecraft:dirt", 64));
        }
        inventory.slots[40] = Some(ItemStack::new("minecraft:shield", 1));
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        let cheytac = index("iw4:weapon/cheytac_mp", &[]);
        let usp = index("iw4:weapon/usp_mp", &[]);
        let picked = class(cheytac, usp);
        let given = give(&mut inventory, &mut tags, Some(&picked), &mut names).unwrap();
        let dropped: Vec<(&str, u8)> = given
            .dropped
            .iter()
            .map(|stack| (stack.id.as_str(), stack.count))
            .collect();
        assert_eq!(
            dropped,
            vec![
                ("iw4l:cheytac", 1),
                ("iw4l:usp", 1),
                (fork_items::SNIPER_ROUNDS, 15),
                (fork_items::PISTOL_AMMO, 36),
            ]
        );
        assert_eq!(tags, vec![GIVEN_TAG.to_owned()]);
    }

    #[test]
    fn a_gun_outside_the_table_becomes_the_first_gun_of_its_category() {
        let mut inventory = Inventory::default();
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        let shotgun = index("iw4:weapon/new_shotgun_mp", &[]);
        let strange = index("t5:weapon/strange_mp", &[]);
        let picked = class(shotgun, strange);
        let given = give(&mut inventory, &mut tags, Some(&picked), &mut names).unwrap();
        assert_eq!(
            given.stood_in,
            vec!["iw4:weapon/new_shotgun_mp", "t5:weapon/strange_mp"]
        );
        let id = |slot: usize| inventory.slots[slot].as_ref().map(|stack| stack.id.clone());
        assert_eq!(id(0).as_deref(), Some("iw4l:m1014"));
        assert_eq!(id(1).as_deref(), Some("iw4l:usp"));
        assert_eq!(count(&inventory, fork_items::SHOTGUN_SHELLS), 12);
        assert_eq!(count(&inventory, fork_items::PISTOL_AMMO), 36);
        assert_eq!(stand_in(Bucket::Special), None);
        for bucket in [
            Bucket::AssaultRifle,
            Bucket::Smg,
            Bucket::Lmg,
            Bucket::Sniper,
            Bucket::Shotgun,
            Bucket::MachinePistol,
            Bucket::Pistol,
            Bucket::Launcher,
        ] {
            let row = fork_items::get(stand_in(bucket).unwrap()).unwrap();
            assert_eq!((row.kind, row.bucket), (Kind::Gun, bucket));
        }
    }

    #[test]
    fn a_new_player_gets_32_of_each_class_grenade_once() {
        let mut inventory = Inventory::default();
        let mut tags = Vec::new();
        let mut names = GunNames::default();
        let picked = Class {
            lethal: index("iw4:weapon/semtex_mp", &[]),
            tactical: index("iw4:weapon/concussion_grenade_mp", &[]),
            ..class(index("iw4:weapon/m4_mp", &[]), 0)
        };
        let given = give(&mut inventory, &mut tags, Some(&picked), &mut names).unwrap();
        assert!(given.dropped.is_empty());
        let slot = |inventory: &Inventory, slot: usize| {
            inventory.slots[slot]
                .as_ref()
                .map(|stack| (stack.id.clone(), stack.count))
        };
        assert_eq!(
            slot(&inventory, fork_items::LETHAL_SLOT),
            Some(("iw4l:semtex".to_owned(), 32))
        );
        assert_eq!(
            slot(&inventory, fork_items::TACTICAL_SLOT),
            Some(("iw4l:concussion_grenade".to_owned(), 32))
        );
        assert!(give(&mut inventory, &mut tags, Some(&picked), &mut names).is_none());
        assert_eq!(count(&inventory, "iw4l:semtex"), 32, "only once");
        let mut fresh = Inventory::default();
        fresh.slots[fork_items::LETHAL_SLOT] = Some(ItemStack::new("iw4l:frag_grenade", 40));
        let plain = class(index("iw4:weapon/usp_mp", &[]), 0);
        give(&mut fresh, &mut Vec::new(), Some(&plain), &mut names).unwrap();
        assert_eq!(
            slot(&fresh, fork_items::LETHAL_SLOT),
            Some(("iw4l:frag_grenade".to_owned(), 64)),
            "no class lethal falls back to frags, topping up the slot"
        );
        assert_eq!(count(&fresh, "iw4l:frag_grenade"), 72);
        assert_eq!(
            slot(&fresh, fork_items::TACTICAL_SLOT),
            Some(("iw4l:flash_grenade".to_owned(), 32))
        );
    }
}
