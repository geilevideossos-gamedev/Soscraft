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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GunHit {
    Bullet,
    Explosion,
    Melee,
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
    ((damage * kept * scale).round() as i32).max(1)
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
    let (Some(kind), Some(guard)) = (gun_hit(means), victim.guard) else {
        return Verdict::Unchanged;
    };
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
        ];
        for (amount, expected) in table {
            let kind = if amount >= 150 {
                GunHit::Explosion
            } else {
                GunHit::Bullet
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
