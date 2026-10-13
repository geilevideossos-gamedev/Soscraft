use crate::voxel::GunGuard;

pub const GUN_PLAYER_SCALE: f64 = 0.5;
pub const GUN_ARMOR_FACTOR: f64 = 2.0;
pub const GUN_PROTECTION_CAP: f64 = 0.8;
pub const PIERCE_LMG: f32 = 0.25;
pub const PIERCE_SNIPER: f32 = 0.5;
pub const PIERCE_FMJ: f32 = 0.25;
pub const PIERCE_MAX: f32 = 0.75;
pub(crate) const DEFAULT_MAX_HEALTH: i32 = 100;

const HEALTH_POINTS: f64 = 20.0;
const ARMOR_CEILING: f64 = 20.0;
const ARMOR_FLOOR: f64 = 0.2;
const ARMOR_DIVISOR: f64 = 25.0;
const ENCHANTMENT_CAP: f64 = 20.0;
const ENCHANTMENT_SHARE: f64 = 0.04;
const RESISTANCE_SHARE: f64 = 0.2;
const ROUNDING_SLACK: f64 = 1e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GunHit {
    Bullet,
    Explosion,
    Melee,
}

impl GunHit {
    pub fn turned_by_totem(self) -> bool {
        matches!(self, Self::Bullet | Self::Explosion)
    }
}

pub fn gun_hit(means: &str) -> Option<GunHit> {
    match means {
        "MOD_PISTOL_BULLET" | "MOD_RIFLE_BULLET" | "MOD_HEAD_SHOT" | "MOD_EXPLOSIVE_BULLET" => {
            Some(GunHit::Bullet)
        }
        "MOD_GRENADE"
        | "MOD_GRENADE_SPLASH"
        | "MOD_PROJECTILE"
        | "MOD_PROJECTILE_SPLASH"
        | "MOD_EXPLOSIVE"
        | "MOD_ARTILLERY" => Some(GunHit::Explosion),
        "MOD_MELEE" | "MOD_IMPACT" => Some(GunHit::Melee),
        _ => None,
    }
}

pub fn pierce(lmg: bool, sniper: bool, fmj: bool) -> f32 {
    let class = if lmg {
        PIERCE_LMG
    } else if sniper {
        PIERCE_SNIPER
    } else {
        0.0
    };
    let fmj = if fmj { PIERCE_FMJ } else { 0.0 };
    (class + fmj).min(PIERCE_MAX)
}

pub fn gun_damage_to_player(
    amount: i32,
    max_health: i32,
    guard: &GunGuard,
    pierce: f32,
    kind: GunHit,
) -> i32 {
    if amount <= 0 {
        return 0;
    }
    let scale = f64::from(max_health.max(1)) / HEALTH_POINTS;
    let damage = f64::from(amount) / scale * GUN_PLAYER_SCALE;
    let armor = f64::from(guard.armor.max(0.0));
    let toughness = 2.0 + f64::from(guard.toughness.max(0.0)) / 4.0;
    let absorbed = (armor - damage / toughness)
        .max(armor * ARMOR_FLOOR)
        .min(ARMOR_CEILING)
        / ARMOR_DIVISOR;
    let armor_share = (GUN_ARMOR_FACTOR * absorbed).min(GUN_PROTECTION_CAP);
    let epf = match kind {
        GunHit::Bullet => guard.epf_bullet,
        GunHit::Explosion => guard.epf_blast,
        GunHit::Melee => guard.epf_melee,
    };
    let enchantment_share = f64::from(epf).clamp(0.0, ENCHANTMENT_CAP) * ENCHANTMENT_SHARE;
    let kept = ((1.0 - armor_share) * (1.0 - enchantment_share)).max(1.0 - GUN_PROTECTION_CAP);
    let pierce = f64::from(pierce.clamp(0.0, PIERCE_MAX));
    let kept = 1.0 - (1.0 - kept) * (1.0 - pierce);
    let resisted = (RESISTANCE_SHARE * f64::from(guard.resistance)).min(1.0);
    let kept = kept * (1.0 - resisted);
    if kept <= 0.0 {
        return 0;
    }
    ((damage * kept * scale + ROUNDING_SLACK).round() as i32).max(1)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Verdict {
    Unchanged,
    Refused,
    Amount(i32),
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Victim {
    pub(crate) guard: Option<GunGuard>,
    pub(crate) max_health: i32,
    pub(crate) totem_grace: bool,
}

pub(crate) fn verdict(
    minecraft: bool,
    means: &str,
    amount: i32,
    pierce: f32,
    victim: Victim,
) -> Verdict {
    if !minecraft || amount <= 0 {
        return Verdict::Unchanged;
    }
    let Some(kind) = gun_hit(means) else {
        return Verdict::Unchanged;
    };
    let guard_grace = victim.guard.is_some_and(|guard| guard.totem_grace);
    if kind.turned_by_totem() && (victim.totem_grace || guard_grace) {
        return Verdict::Refused;
    }
    let Some(guard) = victim.guard else {
        return Verdict::Unchanged;
    };
    let pierce = if kind == GunHit::Bullet { pierce } else { 0.0 };
    match gun_damage_to_player(amount, victim.max_health, &guard, pierce, kind) {
        0 => Verdict::Refused,
        amount => Verdict::Amount(amount),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SETS: [(f32, f32); 8] = [
        (0.0, 0.0),
        (7.0, 0.0),
        (10.0, 0.0),
        (11.0, 0.0),
        (12.0, 0.0),
        (15.0, 0.0),
        (20.0, 8.0),
        (20.0, 12.0),
    ];

    fn worn(armor: f32, toughness: f32) -> GunGuard {
        GunGuard {
            armor,
            toughness,
            ..GunGuard::default()
        }
    }

    fn row(amount: i32, pierce: f32, kind: GunHit) -> Vec<i32> {
        SETS.iter()
            .map(|&(armor, toughness)| {
                gun_damage_to_player(amount, 100, &worn(armor, toughness), pierce, kind)
            })
            .collect()
    }

    fn victim(guard: GunGuard) -> Victim {
        Victim {
            guard: Some(guard),
            max_health: 100,
            totem_grace: false,
        }
    }

    #[test]
    fn gun_hits_are_bullets_explosions_or_melee() {
        for means in [
            "MOD_PISTOL_BULLET",
            "MOD_RIFLE_BULLET",
            "MOD_HEAD_SHOT",
            "MOD_EXPLOSIVE_BULLET",
        ] {
            assert_eq!(gun_hit(means), Some(GunHit::Bullet), "{means}");
        }
        for means in [
            "MOD_GRENADE",
            "MOD_GRENADE_SPLASH",
            "MOD_PROJECTILE",
            "MOD_PROJECTILE_SPLASH",
            "MOD_EXPLOSIVE",
            "MOD_ARTILLERY",
        ] {
            assert_eq!(gun_hit(means), Some(GunHit::Explosion), "{means}");
        }
        for means in ["MOD_MELEE", "MOD_IMPACT"] {
            assert_eq!(gun_hit(means), Some(GunHit::Melee), "{means}");
        }
        for means in [
            crate::script_player::MINECRAFT_MEANS,
            "MOD_FALLING",
            "MOD_SUICIDE",
            "MOD_CRUSH",
            "MOD_UNKNOWN",
            "",
        ] {
            assert_eq!(gun_hit(means), None, "{means}");
        }
    }

    #[test]
    fn guns_do_half_damage_to_an_unarmoured_player() {
        let bare = GunGuard::default();
        for (amount, kept) in [(30, 15), (98, 49), (130, 65), (1, 1)] {
            for kind in [GunHit::Bullet, GunHit::Explosion, GunHit::Melee] {
                assert_eq!(
                    gun_damage_to_player(amount, 100, &bare, 0.0, kind),
                    kept,
                    "{amount} {kind:?}"
                );
            }
        }
        assert_eq!(
            gun_damage_to_player(30, 400, &bare, 0.0, GunHit::Bullet),
            15
        );
        assert_eq!(gun_damage_to_player(0, 100, &bare, 0.0, GunHit::Bullet), 0);
    }

    #[test]
    fn armour_takes_twice_vanilla_s_share_up_to_four_fifths() {
        let table = [
            (30, [15, 8, 5, 4, 3, 3, 3, 3]),
            (40, [20, 12, 7, 6, 4, 4, 4, 4]),
            (50, [25, 16, 10, 8, 6, 5, 5, 5]),
            (150, [75, 67, 60, 54, 48, 30, 15, 15]),
            (160, [80, 71, 67, 61, 54, 35, 16, 16]),
            (200, [100, 89, 84, 82, 81, 60, 20, 20]),
            (135, [68, 60, 50, 45, 39, 23, 14, 14]),
        ];
        for (amount, expected) in table {
            let kind = match amount {
                135 => GunHit::Melee,
                150.. => GunHit::Explosion,
                _ => GunHit::Bullet,
            };
            assert_eq!(row(amount, 0.0, kind), expected, "{amount}");
        }
        let best = GunGuard {
            epf_bullet: 20.0,
            ..worn(20.0, 12.0)
        };
        assert_eq!(
            gun_damage_to_player(30, 100, &best, 0.0, GunHit::Bullet),
            3,
            "never more than four fifths taken away"
        );
        assert_eq!(
            gun_damage_to_player(1000, 100, &worn(20.0, 8.0), 0.0, GunHit::Explosion),
            340,
            "a heavy hit gets through more armour, like vanilla"
        );
    }

    #[test]
    fn an_x_5_result_at_the_ceiling_rounds_up() {
        let at_ceiling = [
            (135, (20.0, 8.0), GunHit::Melee, 14),
            (135, (20.0, 12.0), GunHit::Melee, 14),
            (25, (12.0, 0.0), GunHit::Bullet, 3),
            (55, (20.0, 8.0), GunHit::Explosion, 6),
            (155, (20.0, 8.0), GunHit::Explosion, 16),
        ];
        for (amount, (armor, toughness), kind, expected) in at_ceiling {
            assert_eq!(
                gun_damage_to_player(amount, 100, &worn(armor, toughness), 0.0, kind),
                expected,
                "{amount} against {armor}"
            );
        }
        assert_eq!(
            gun_damage_to_player(105, 100, &GunGuard::default(), 0.0, GunHit::Bullet),
            53
        );
    }

    #[test]
    fn protection_and_resistance_count_under_the_same_ceiling() {
        let protected = GunGuard {
            epf_bullet: 16.0,
            epf_blast: 16.0,
            epf_melee: 16.0,
            ..GunGuard::default()
        };
        assert_eq!(
            gun_damage_to_player(30, 100, &protected, 0.0, GunHit::Bullet),
            5
        );
        assert_eq!(
            gun_damage_to_player(150, 100, &protected, 0.0, GunHit::Explosion),
            27
        );
        let blast = GunGuard {
            epf_blast: 32.0,
            ..GunGuard::default()
        };
        assert_eq!(
            gun_damage_to_player(150, 100, &blast, 0.0, GunHit::Explosion),
            15
        );
        assert_eq!(
            gun_damage_to_player(30, 100, &blast, 0.0, GunHit::Bullet),
            15
        );
        let iron_protected = GunGuard {
            epf_bullet: 16.0,
            ..worn(15.0, 0.0)
        };
        assert_eq!(
            gun_damage_to_player(30, 100, &iron_protected, 0.0, GunHit::Bullet),
            3
        );
        let resistant = GunGuard {
            resistance: 2,
            ..GunGuard::default()
        };
        assert_eq!(
            gun_damage_to_player(30, 100, &resistant, 0.0, GunHit::Bullet),
            9
        );
        let iron_resistant = GunGuard {
            resistance: 2,
            ..worn(15.0, 0.0)
        };
        assert_eq!(
            gun_damage_to_player(150, 100, &iron_resistant, 0.0, GunHit::Explosion),
            18
        );
        let immune = GunGuard {
            resistance: 5,
            ..GunGuard::default()
        };
        assert_eq!(
            gun_damage_to_player(1000, 100, &immune, 0.75, GunHit::Bullet),
            0
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, victim(immune)),
            Verdict::Refused
        );
    }

    #[test]
    fn pierce_cuts_the_protection() {
        assert_eq!(pierce(false, false, false), 0.0);
        assert_eq!(pierce(true, false, false), PIERCE_LMG);
        assert_eq!(pierce(false, true, false), PIERCE_SNIPER);
        assert_eq!(pierce(false, false, true), PIERCE_FMJ);
        assert_eq!(pierce(true, false, true), 0.5);
        assert_eq!(pierce(false, true, true), PIERCE_MAX);
        let table: [(i32, f32, [Option<i32>; 8]); 6] = [
            (
                30,
                pierce(false, false, true),
                [15, 10, 7, 6, 6, 6, 6, 6].map(Some),
            ),
            (
                40,
                pierce(true, false, false),
                [20, 14, 10, 9, 8, 8, 8, 8].map(Some),
            ),
            (
                40,
                pierce(true, false, true),
                [20, 16, 14, 13, 12, 12, 12, 12].map(Some),
            ),
            (
                30,
                pierce(true, false, false),
                [15, 10, 7, 6, 6, 6, 6, 6].map(Some),
            ),
            (
                105,
                pierce(false, true, false),
                [
                    None,
                    Some(49),
                    Some(43),
                    Some(40),
                    Some(38),
                    Some(32),
                    None,
                    None,
                ],
            ),
            (
                105,
                pierce(false, true, true),
                [
                    None,
                    Some(51),
                    Some(48),
                    Some(46),
                    Some(45),
                    Some(42),
                    Some(42),
                    Some(42),
                ],
            ),
        ];
        for (amount, pierce, expected) in table {
            for (got, want) in row(amount, pierce, GunHit::Bullet)
                .into_iter()
                .zip(expected)
            {
                if let Some(want) = want {
                    assert_eq!(got, want, "{amount} pierce {pierce}");
                }
            }
        }
        let iron = worn(15.0, 0.0);
        assert_eq!(
            gun_damage_to_player(30, 100, &iron, 1.0, GunHit::Bullet),
            12
        );
        let immune = GunGuard {
            resistance: 5,
            ..iron
        };
        assert_eq!(
            gun_damage_to_player(105, 100, &immune, 0.75, GunHit::Bullet),
            0
        );
    }

    #[test]
    fn a_spent_totem_turns_guns_away_for_four_seconds() {
        let client = 7_000_101;
        crate::voxel::set_death_protected(client, true);
        assert!(crate::voxel::spend_death_protection(
            client,
            crate::Tick(100)
        ));
        let at = |tick: u32| Victim {
            totem_grace: crate::voxel::totem_grace(client, crate::Tick(tick)),
            ..victim(GunGuard::default())
        };
        for tick in [100, 179] {
            for means in [
                "MOD_RIFLE_BULLET",
                "MOD_HEAD_SHOT",
                "MOD_GRENADE_SPLASH",
                "MOD_PROJECTILE",
            ] {
                assert_eq!(
                    verdict(true, means, 30, 0.0, at(tick)),
                    Verdict::Refused,
                    "{tick} {means}"
                );
            }
        }
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, at(180)),
            Verdict::Amount(15)
        );
        assert_eq!(
            verdict(true, "MOD_GRENADE_SPLASH", 150, 0.0, at(180)),
            Verdict::Amount(75)
        );
        assert!(
            !crate::voxel::totem_grace(7_000_102, crate::Tick(100)),
            "only the popped player"
        );
        let mirrored = victim(GunGuard {
            totem_grace: true,
            ..GunGuard::default()
        });
        assert!(!crate::voxel::totem_grace(client, crate::Tick(500)));
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, mirrored),
            Verdict::Refused
        );
        assert_eq!(
            verdict(false, "MOD_RIFLE_BULLET", 30, 0.0, mirrored),
            Verdict::Unchanged
        );
        assert_eq!(
            crate::voxel::take_totem_pops()
                .iter()
                .filter(|&&c| c == client)
                .count(),
            1
        );
    }

    #[test]
    fn a_knife_still_hurts_during_the_grace() {
        let graced = Victim {
            totem_grace: true,
            ..victim(GunGuard::default())
        };
        assert_eq!(
            verdict(true, "MOD_MELEE", 30, 0.0, graced),
            Verdict::Amount(15)
        );
        assert_eq!(
            verdict(true, "MOD_IMPACT", 30, 0.0, graced),
            Verdict::Amount(15)
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, graced),
            Verdict::Refused
        );
        assert_eq!(
            verdict(true, crate::script_player::MINECRAFT_MEANS, 30, 0.0, graced),
            Verdict::Unchanged
        );
    }

    #[test]
    fn only_bullets_pierce_whatever_gun_is_held() {
        let iron = victim(worn(15.0, 0.0));
        let sniper = pierce(false, true, true);
        assert_eq!(
            verdict(true, "MOD_MELEE", 135, sniper, iron),
            Verdict::Amount(23),
            "a knife lunge with a sniper in hand"
        );
        assert_eq!(
            verdict(true, "MOD_MELEE", 135, sniper, iron),
            verdict(true, "MOD_MELEE", 135, 0.0, iron)
        );
        assert_eq!(
            verdict(true, "MOD_GRENADE_SPLASH", 150, PIERCE_LMG, iron),
            verdict(true, "MOD_GRENADE_SPLASH", 150, 0.0, iron)
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 105, sniper, iron),
            Verdict::Amount(42)
        );
    }

    #[test]
    fn mw2_maps_pass_the_amount_through() {
        let bare = victim(GunGuard::default());
        assert_eq!(
            verdict(false, "MOD_RIFLE_BULLET", 30, 0.0, bare),
            Verdict::Unchanged
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, bare),
            Verdict::Amount(15)
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 30, 0.0, Victim::default()),
            Verdict::Unchanged,
            "a player that never published its armour, like a bot"
        );
        assert_eq!(
            verdict(true, crate::script_player::MINECRAFT_MEANS, 30, 0.0, bare),
            Verdict::Unchanged
        );
        assert_eq!(
            verdict(true, "MOD_FALLING", 30, 0.0, bare),
            Verdict::Unchanged
        );
        assert_eq!(
            verdict(true, "MOD_RIFLE_BULLET", 0, 0.0, bare),
            Verdict::Unchanged
        );
    }

    #[test]
    fn mob_damage_is_unchanged() {
        let rule = "gun_damage::";
        assert!(include_str!("script/host/players.rs").contains(rule));
        for (name, source) in [
            ("damage.rs", include_str!("damage.rs")),
            ("voxel.rs", include_str!("voxel.rs")),
            ("script_player.rs", include_str!("script_player.rs")),
            ("combat.rs", include_str!("combat.rs")),
        ] {
            assert!(!source.contains(rule), "{name} applies the gun rule");
        }
    }
}
