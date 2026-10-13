use super::*;
use crate::voxel::{AMMO_TYPES, Arsenal, GunSpec, NO_AMMO, Published, Spent};
use weapon_iw4::WeaponCombatFacts;

const M4: u32 = 1;
const GL: u32 = 2;
const USP: u32 = 3;
const FRAG: u32 = 4;
const AC130: u32 = 5;
const CHEYTAC: u32 = 6;
const SPAS: u32 = 7;
const UMP: u32 = 8;
const AK: u32 = 9;
const AC130_GUN: u32 = 10;
const ME: ClientId = ClientId(7);

fn facts(clip: i32, max_ammo: i32, inventory_type: i32) -> WeaponCombatFacts {
    WeaponCombatFacts {
        clip_size: clip,
        start_ammo: clip,
        max_ammo,
        fire_time_ms: 100,
        raise_time_ms: 300,
        inventory_type,
        ..WeaponCombatFacts::none()
    }
}

fn sim() -> crate::SimWorld {
    let names = [
        "none",
        "m4_gl_mp",
        "gl_m4_mp",
        "usp_mp",
        "frag_grenade_mp",
        "killstreak_ac130_mp",
        "cheytac_mp",
        "spas12_mp",
        "ump45_mp",
        "ak47_mp",
        "ac130_105mm_mp",
    ];
    let mut rows = vec![
        WeaponCombatFacts::none(),
        WeaponCombatFacts {
            alternate_weapon: GL,
            ..facts(30, 120, 0)
        },
        facts(1, 1, 3),
        facts(12, 48, 0),
        facts(1, 1, 1),
        facts(1, 0, 0),
    ];
    rows.extend([facts(5, 20, 0), facts(8, 32, 0), facts(32, 128, 0)]);
    rows.extend([facts(30, 120, 0), facts(1, 99, 0)]);
    let mut builder = crate::SimContentBuilder::default();
    builder.set_weapon_script_names(names.iter().map(|name| name.to_string()).collect());
    builder.set_weapon_combat_table(rows);
    let mut sim = crate::SimWorld::default();
    sim.install_content(builder.finish());
    sim
}

fn carrying(sim: &mut crate::SimWorld, weapons: &[u32], held: u32) {
    let mut frame = sim.frame();
    let ps = frame.ensure_player(ME);
    for &weapon in weapons {
        inventory_add_weapon(ps, weapon, false);
    }
    ps.weapon = held;
    ps.weapon_primary = held;
    frame.client_meta_mut(ME).lifecycle = ClientLifecycle::Alive;
}

fn gun(weapon: u32, clip: i32) -> GunSpec {
    GunSpec {
        weapon,
        clip,
        clip_left: 0,
        alt_clip: 0,
        ammo: NO_AMMO,
        rebind: false,
        alt_ammo: NO_AMMO,
    }
}

fn published(guns: Vec<GunSpec>, selected: Option<u32>) -> Vec<(u32, Published)> {
    let arsenal = Arsenal {
        guns,
        selected,
        ..Arsenal::default()
    };
    vec![(
        ME.0,
        Published {
            arsenal,
            ..Published::default()
        },
    )]
}

fn step(sim: &mut crate::SimWorld, published: &mut [(u32, Published)]) -> Vec<Spent> {
    step_with(sim, published, &[])
}

fn step_with(
    sim: &mut crate::SimWorld,
    published: &mut [(u32, Published)],
    queued: &[(u32, Spent)],
) -> Vec<Spent> {
    let mut frame = sim.frame();
    apply_arsenal_with(&mut frame, published, queued)
        .into_iter()
        .map(|(client, spent)| {
            assert_eq!(client, ME.0);
            spent
        })
        .collect()
}

fn weapons(sim: &mut crate::SimWorld) -> Vec<u32> {
    let mut held: Vec<u32> = sim
        .frame()
        .player(ME)
        .expect("the player")
        .weapons
        .iter()
        .filter(|&&w| w > 0)
        .map(|&w| w as u32)
        .collect();
    held.sort_unstable();
    held
}

fn ammo(sim: &mut crate::SimWorld, weapon: u32) -> (i32, i32, i32) {
    let frame = sim.frame();
    let ps = *frame.player(ME).expect("the player");
    ammo_from_ps(&frame, &ps, weapon)
}

fn set_clip(sim: &mut crate::SimWorld, weapon: u32, clip: i32) {
    let mut frame = sim.frame();
    let mut ps = *frame.player(ME).expect("the player");
    let (_, left, stock) = ammo_from_ps(&frame, &ps, weapon);
    set_ammo_on_ps(&frame, &mut ps, weapon, clip, left, stock);
    *frame.player_mut(ME).expect("the player") = ps;
}

#[test]
fn the_published_guns_become_the_primaries() {
    let mut sim = sim();
    carrying(&mut sim, &[AK, M4, GL, FRAG, AC130, AC130_GUN], AK);
    let mut published = published(vec![gun(USP, 7), gun(M4, 30)], Some(USP));
    assert!(step(&mut sim, &mut published).is_empty());
    assert_eq!(
        weapons(&mut sim),
        vec![M4, GL, USP, FRAG, AC130, AC130_GUN],
        "offhands, alt modes, killstreaks and the AC-130's guns stay"
    );
    let ps = *sim.frame().player(ME).expect("the player");
    assert_eq!(
        ps.weapon, USP,
        "the removed held gun switches to the selected one"
    );
    assert_eq!(ps.weapon_primary, USP);
    assert_eq!(ammo(&mut sim, USP).0, 7);
    assert_eq!(ammo(&mut sim, M4).0, 30);
    published[0].1.arsenal = Arsenal {
        guns: vec![gun(M4, 30)],
        selected: None,
        ..Arsenal::default()
    };
    step(&mut sim, &mut published);
    assert_eq!(weapons(&mut sim), vec![M4, GL, FRAG, AC130, AC130_GUN]);
    assert_eq!(sim.frame().player(ME).expect("the player").weapon, 0);
}

#[test]
fn no_intervention_spas_or_ump_is_added() {
    let mut sim = sim();
    carrying(&mut sim, &[M4], M4);
    step(&mut sim, &mut Vec::new());
    assert_eq!(weapons(&mut sim), vec![M4]);
    let mut published = published(vec![gun(M4, 30)], Some(M4));
    step(&mut sim, &mut published);
    let held = weapons(&mut sim);
    for weapon in [CHEYTAC, SPAS, UMP] {
        assert!(!held.contains(&weapon));
    }
}

#[test]
fn a_client_that_never_published_keeps_the_full_refill() {
    let mut sim = sim();
    carrying(&mut sim, &[M4, FRAG, USP], M4);
    set_clip(&mut sim, M4, 3);
    set_clip(&mut sim, FRAG, 0);
    assert!(step(&mut sim, &mut Vec::new()).is_empty());
    assert_eq!(ammo(&mut sim, M4), (30, 0, 120));
    assert_eq!(ammo(&mut sim, FRAG).0, 1);
    assert_eq!(ammo(&mut sim, USP), (12, 0, 48));
    assert_eq!(weapons(&mut sim), vec![M4, USP, FRAG]);
}

#[test]
fn a_held_gun_queues_its_clip() {
    let mut sim = sim();
    carrying(&mut sim, &[M4, USP], M4);
    let mut published = published(
        vec![
            GunSpec {
                alt_clip: 1,
                ..gun(M4, 30)
            },
            gun(USP, -1),
        ],
        Some(M4),
    );
    assert_eq!(
        step(&mut sim, &mut published),
        vec![Spent::Clip {
            weapon: USP,
            clip: 12,
            clip_left: 0,
            alt_clip: 0,
        }],
        "a full clip of unknown size is reported with its real size"
    );
    assert_eq!(ammo(&mut sim, M4).0, 30);
    assert_eq!(ammo(&mut sim, GL).0, 1);
    assert_eq!(ammo(&mut sim, USP).0, 12);
    set_clip(&mut sim, M4, 17);
    set_clip(&mut sim, USP, 2);
    assert_eq!(
        step(&mut sim, &mut published),
        vec![Spent::Clip {
            weapon: M4,
            clip: 17,
            clip_left: 0,
            alt_clip: 1,
        }]
    );
    assert_eq!(ammo(&mut sim, M4).0, 17, "the held gun keeps its live clip");
    assert_eq!(
        ammo(&mut sim, USP).0,
        12,
        "a gun not held gets its spec clip"
    );
    assert!(step(&mut sim, &mut published).is_empty(), "queued once");
    published[0].1.arsenal.guns[0] = GunSpec {
        rebind: true,
        alt_clip: 1,
        ..gun(M4, 25)
    };
    assert!(step(&mut sim, &mut published).is_empty());
    assert_eq!(
        ammo(&mut sim, M4).0,
        25,
        "a rebind loads the other copy's clip"
    );
    assert!(!published[0].1.arsenal.guns[0].rebind);
    published[0].1.arsenal.endless = true;
    set_clip(&mut sim, M4, 4);
    assert_eq!(
        step(&mut sim, &mut published),
        vec![Spent::Clip {
            weapon: M4,
            clip: 4,
            clip_left: 0,
            alt_clip: 1,
        }]
    );
    assert_eq!(
        ammo(&mut sim, M4),
        (4, 0, 120),
        "in creative the clip still empties and the stock is full"
    );
}

const RIFLE: u8 = 1;
const EXPLOSIVE: u8 = 5;

fn rifle_gun(weapon: u32, clip: i32) -> GunSpec {
    GunSpec {
        ammo: RIFLE,
        alt_ammo: EXPLOSIVE,
        ..gun(weapon, clip)
    }
}

fn stocked(guns: Vec<GunSpec>, stock: [i32; AMMO_TYPES]) -> Vec<(u32, Published)> {
    let mut published = published(guns, Some(M4));
    published[0].1.arsenal.stock = stock;
    published
}

fn rounds(client: ClientId, ammo: u8, rounds: i32) -> (u32, Spent) {
    (client.0, Spent::Rounds { ammo, rounds })
}

#[test]
fn the_stock_is_the_counted_ammo_less_the_rounds_not_yet_taken() {
    let mut sim = sim();
    carrying(&mut sim, &[M4, USP], M4);
    let mut stock = [0; AMMO_TYPES];
    stock[usize::from(RIFLE)] = 50;
    stock[usize::from(EXPLOSIVE)] = 3;
    let mut published = stocked(vec![rifle_gun(M4, 30)], stock);
    let queued = [
        rounds(ME, RIFLE, 18),
        rounds(ClientId(ME.0 + 1), RIFLE, 7),
        rounds(ME, EXPLOSIVE, 2),
    ];
    step_with(&mut sim, &mut published, &queued);
    assert_eq!(
        ammo(&mut sim, M4),
        (30, 0, 32),
        "50 rifle rounds less the 18 still queued"
    );
    assert_eq!(
        ammo(&mut sim, GL).2,
        1,
        "the grenade launcher counts its own type"
    );
    published[0].1.arsenal.stock[usize::from(RIFLE)] = 500;
    step(&mut sim, &mut published);
    assert_eq!(ammo(&mut sim, M4).2, 120, "never more than maxAmmo");
    published[0].1.arsenal.stock = [0; AMMO_TYPES];
    published[0].1.arsenal.endless = true;
    step(&mut sim, &mut published);
    assert_eq!(ammo(&mut sim, M4).2, 120, "creative has a full stock");
}

#[test]
fn no_ammo_items_leave_the_gun_to_click_empty() {
    let mut sim = sim();
    carrying(&mut sim, &[M4], M4);
    let mut published = stocked(vec![rifle_gun(M4, 0)], [0; AMMO_TYPES]);
    step(&mut sim, &mut published);
    assert_eq!(ammo(&mut sim, M4), (0, 0, 0));
    let hand = weapon_iw4::WeaponHandState::default();
    let facts = sim.frame().combat_facts_for(M4).expect("the M4");
    assert!(!weapon_iw4::weapon_allow_reload(&hand, &facts));
}

#[test]
fn a_spawn_refill_creates_no_items() {
    let mut sim = sim();
    carrying(&mut sim, &[M4, USP], M4);
    let mut stock = [0; AMMO_TYPES];
    stock[usize::from(RIFLE)] = 10;
    let mut published = stocked(vec![rifle_gun(M4, 5), rifle_gun(USP, 2)], stock);
    step(&mut sim, &mut published);
    assert_eq!(ammo(&mut sim, M4), (5, 0, 10));
    {
        let mut frame = sim.frame();
        crate::script_player::take_all_weapons(&mut frame, ME);
        for weapon in [M4, USP] {
            crate::script_player::give_weapon(&mut frame, ME, weapon, false).expect("given");
            crate::script_player::give_max_ammo(&mut frame, ME, weapon);
        }
        let ps = frame.player_mut(ME).expect("the player");
        ps.weapon = M4;
        ps.weapon_primary = M4;
    }
    assert_eq!(ammo(&mut sim, M4).0, 30, "giveLoadout filled the clip");
    published[0].1.scripted = true;
    assert!(step(&mut sim, &mut published).is_empty(), "no clip grows");
    assert_eq!(ammo(&mut sim, M4), (5, 0, 10));
    assert_eq!(ammo(&mut sim, USP), (2, 0, 10));
    assert!(!published[0].1.scripted);
    set_clip(&mut sim, M4, 3);
    published[0].1.scripted = true;
    assert_eq!(
        step(&mut sim, &mut published),
        vec![Spent::Clip {
            weapon: M4,
            clip: 3,
            clip_left: 0,
            alt_clip: 0,
        }],
        "rounds fired before a script call stay spent"
    );
}

#[test]
fn script_clip_changes_mark_only_a_publishing_client() {
    let mine = ClientId(4241);
    let other = ClientId(4242);
    crate::voxel::set_arsenal(mine.0, Arsenal::default());
    let mut sim = sim();
    {
        let mut frame = sim.frame();
        crate::script_player::set_ammo_clip(&mut frame, mine, M4, 3);
        crate::script_player::set_ammo_clip(&mut frame, other, M4, 3);
    }
    let marked = crate::voxel::with_arsenals(|published| {
        let marked: Vec<u32> = published
            .iter()
            .filter(|(client, held)| held.scripted && [mine.0, other.0].contains(client))
            .map(|(client, _)| *client)
            .collect();
        (marked, Vec::new())
    });
    crate::voxel::forget_arsenal(mine.0);
    assert_eq!(marked, Some(vec![mine.0]));
}

#[test]
fn a_reload_queues_rounds_of_the_gun_s_ammo_type() {
    let client = 4243;
    let alternate = |weapon: u32| if weapon == M4 { GL } else { 0 };
    crate::voxel::set_arsenal(
        client,
        Arsenal {
            guns: vec![rifle_gun(M4, 30), gun(USP, 12)],
            ..Arsenal::default()
        },
    );
    crate::voxel::spend_rounds(client, M4, 18, alternate);
    crate::voxel::spend_rounds(client, GL, 1, alternate);
    crate::voxel::spend_rounds(client, USP, 5, alternate);
    crate::voxel::spend_rounds(client, AK, 5, alternate);
    crate::voxel::spend_rounds(client, M4, 0, alternate);
    let spent = crate::voxel::take_spent(client);
    crate::voxel::set_arsenal(
        client,
        Arsenal {
            guns: vec![rifle_gun(M4, 30)],
            endless: true,
            ..Arsenal::default()
        },
    );
    crate::voxel::spend_rounds(client, M4, 18, alternate);
    let creative = crate::voxel::take_spent(client);
    crate::voxel::forget_arsenal(client);
    assert_eq!(
        spent,
        vec![
            Spent::Rounds {
                ammo: RIFLE,
                rounds: 18,
            },
            Spent::Rounds {
                ammo: EXPLOSIVE,
                rounds: 1,
            },
        ]
    );
    assert!(creative.is_empty(), "creative takes no items");
}
