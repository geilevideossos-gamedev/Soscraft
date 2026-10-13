use super::*;
use crate::voxel::{Arsenal, GunSpec, NO_AMMO, Published, Spent};
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
    }
}

fn published(guns: Vec<GunSpec>, selected: Option<u32>) -> Vec<(u32, Published)> {
    let arsenal = Arsenal {
        guns,
        selected,
        ..Arsenal::default()
    };
    vec![(ME.0, Published { arsenal, held: 0 })]
}

fn step(sim: &mut crate::SimWorld, published: &mut [(u32, Published)]) -> Vec<Spent> {
    let mut frame = sim.frame();
    apply_arsenal_with(&mut frame, published)
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
    assert!(step(&mut sim, &mut published).is_empty());
    assert_eq!(ammo(&mut sim, M4).0, 30, "creative keeps every clip full");
}
