use asset_game::WeaponBodyFacts;

const BLOCK: f32 = sim::voxel::BLOCK;
const DIRT: f32 = 0.5;
const STONE: f32 = 1.5;
const UPPER_TORSO: usize = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Tone {
    Plain,
    Header,
    Value,
}

pub(crate) fn decimal(value: f64) -> String {
    let text = format!("{value:.2}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

pub(crate) fn attack_lines(attack: [f32; 2]) -> Vec<(String, Tone)> {
    vec![
        ("When in Main Hand:".to_owned(), Tone::Header),
        (
            format!(" {} Attack Damage", decimal(f64::from(attack[0]))),
            Tone::Value,
        ),
        (
            format!(" {} Attack Speed", decimal(f64::from(attack[1]))),
            Tone::Value,
        ),
    ]
}

pub(crate) fn hits_to_break(per_hit: f32, hardness: f32) -> u32 {
    if hardness <= 0.0 {
        return 1;
    }
    if per_hit <= 0.0 {
        return 0;
    }
    (hardness / per_hit - 1.0e-4).ceil().max(1.0) as u32
}

fn blocks(units: f32) -> String {
    decimal(f64::from(units / BLOCK).round())
}

pub(crate) fn gun_lines(facts: &WeaponBodyFacts) -> Vec<(String, Tone)> {
    let mut out = Vec::new();
    if facts.weap_type != weapon_iw4::WEAPTYPE_BULLET {
        if facts.explosion_inner_damage > 0 || facts.explosion_outer_damage > 0 {
            out.push((
                format!(
                    "EXPLOSION {} - {}",
                    facts.explosion_inner_damage, facts.explosion_outer_damage
                ),
                Tone::Plain,
            ));
            out.push((
                format!("RADIUS {} BLOCKS", blocks(facts.explosion_radius as f32)),
                Tone::Plain,
            ));
            out.push(("BREAKS BLOCKS AS TNT DOES".to_owned(), Tone::Value));
        }
        return out;
    }
    let spread = facts.weap_class == weapon_iw4::WEAPCLASS_SPREAD;
    let pellets = if spread && facts.shots_per_fire > 1 {
        format!(" x{} PELLETS", facts.shots_per_fire)
    } else {
        String::new()
    };
    let min = if facts.min_damage > 0 {
        facts.min_damage
    } else {
        facts.damage
    };
    let (near, far) = (facts.max_damage_range, facts.min_damage_range);
    let falloff = min != facts.damage && near < far;
    let damage = if falloff {
        format!("DAMAGE {} - {}{pellets}", facts.damage, min)
    } else {
        format!("DAMAGE {}{pellets}", facts.damage)
    };
    out.push((damage, Tone::Plain));
    if spread && far > 0.0 {
        out.push((format!("RANGE {} BLOCKS", blocks(far)), Tone::Plain));
    }
    if falloff {
        out.push((
            format!("FALLOFF {} - {} BLOCKS", blocks(near), blocks(far)),
            Tone::Plain,
        ));
    }
    let body = facts
        .location_damage_mult
        .map_or(1.0, |mult| mult[UPPER_TORSO])
        .max(1.0);
    let per_hit = facts.damage as f32 * body / sim::voxel::DAMAGE_PER_HARDNESS;
    out.push((
        format!("BLOCK DAMAGE {} PER HIT", decimal(f64::from(per_hit))),
        Tone::Value,
    ));
    out.push((
        format!(
            "DIRT {} HITS  STONE {} HITS",
            hits_to_break(per_hit, DIRT),
            hits_to_break(per_hit, STONE)
        ),
        Tone::Value,
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rifle() -> WeaponBodyFacts {
        WeaponBodyFacts {
            weap_type: weapon_iw4::WEAPTYPE_BULLET,
            shots_per_fire: 1,
            damage: 40,
            min_damage: 30,
            max_damage_range: 1000.0,
            min_damage_range: 1800.0,
            ..WeaponBodyFacts::default()
        }
    }

    #[test]
    fn decimals_read_as_vanilla_tooltips_do() {
        assert_eq!(decimal(7.0), "7");
        assert_eq!(decimal(1.5999999046325684), "1.6");
        assert_eq!(decimal(0.25), "0.25");
        assert_eq!(decimal(-0.0001), "0");
    }

    #[test]
    fn a_diamond_sword_hits_for_seven_at_one_point_six() {
        let lines = attack_lines([7.0, 1.6]);
        assert_eq!(lines[0].0, "When in Main Hand:");
        assert_eq!(lines[1].0, " 7 Attack Damage");
        assert_eq!(lines[2].0, " 1.6 Attack Speed");
        assert_eq!(lines[1].1, Tone::Value);
    }

    #[test]
    fn hits_to_break_follow_the_mining_rule() {
        assert_eq!(hits_to_break(0.25, 0.5), 2);
        assert_eq!(hits_to_break(0.25, 1.5), 6);
        assert_eq!(hits_to_break(0.25, 50.0), 200);
        assert_eq!(hits_to_break(0.3, 0.0), 1);
    }

    #[test]
    fn a_rifle_shows_its_falloff_range_and_block_damage() {
        let lines: Vec<String> = gun_lines(&rifle()).into_iter().map(|l| l.0).collect();
        assert_eq!(lines[0], "DAMAGE 40 - 30");
        assert_eq!(lines[1], "FALLOFF 28 - 50 BLOCKS");
        assert_eq!(lines[2], "BLOCK DAMAGE 0.25 PER HIT");
        assert_eq!(lines[3], "DIRT 2 HITS  STONE 6 HITS");
    }

    #[test]
    fn a_rifle_without_falloff_has_no_range_line() {
        let sniper = WeaponBodyFacts {
            damage: 98,
            min_damage: 98,
            ..rifle()
        };
        let lines: Vec<String> = gun_lines(&sniper).into_iter().map(|l| l.0).collect();
        assert_eq!(lines[0], "DAMAGE 98");
        assert!(lines[1].starts_with("BLOCK DAMAGE"));
        let flat = WeaponBodyFacts {
            max_damage_range: 1800.0,
            ..rifle()
        };
        let lines: Vec<String> = gun_lines(&flat).into_iter().map(|l| l.0).collect();
        assert_eq!(lines[0], "DAMAGE 40");
        assert!(lines[1].starts_with("BLOCK DAMAGE"));
    }

    #[test]
    fn a_shotgun_counts_pellets() {
        let shotgun = WeaponBodyFacts {
            weap_class: weapon_iw4::WEAPCLASS_SPREAD,
            shots_per_fire: 8,
            damage: 20,
            min_damage: 20,
            min_damage_range: 400.0,
            ..rifle()
        };
        let lines: Vec<String> = gun_lines(&shotgun).into_iter().map(|l| l.0).collect();
        assert_eq!(lines[0], "DAMAGE 20 x8 PELLETS");
        assert_eq!(lines[1], "RANGE 11 BLOCKS");
        assert!(lines[2].starts_with("BLOCK DAMAGE"));
    }

    #[test]
    fn launchers_show_their_blast() {
        let launcher = WeaponBodyFacts {
            weap_type: weapon_iw4::WEAPTYPE_PROJECTILE,
            explosion_inner_damage: 200,
            explosion_outer_damage: 50,
            explosion_radius: 256,
            ..WeaponBodyFacts::default()
        };
        let lines: Vec<String> = gun_lines(&launcher).into_iter().map(|l| l.0).collect();
        assert_eq!(
            lines,
            vec![
                "EXPLOSION 200 - 50",
                "RADIUS 7 BLOCKS",
                "BREAKS BLOCKS AS TNT DOES"
            ]
        );
    }
}
