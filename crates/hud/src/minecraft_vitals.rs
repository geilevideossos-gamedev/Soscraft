#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Bubble {
    Full,
    Bursting,
    Empty,
    Hidden,
}

impl Bubble {
    pub(crate) fn sprite(self) -> Option<&'static str> {
        match self {
            Self::Full => Some("gui:hud/air"),
            Self::Bursting => Some("gui:hud/air_bursting"),
            Self::Empty => Some("gui:hud/air_empty"),
            Self::Hidden => None,
        }
    }
}

fn bubble_count(air: i32, max_air: i32) -> i32 {
    ((air * 10) as f32 / max_air.max(1) as f32).ceil() as i32
}

pub(crate) fn bubbles(air: i32, max_air: i32, eye_in_water: bool) -> Option<[Bubble; 10]> {
    let max = max_air.max(1);
    let air = air.clamp(0, max);
    if !eye_in_water && air >= max {
        return None;
    }
    let full = bubble_count(air - 2, max);
    let popping = bubble_count(air, max);
    let delay = i32::from(air != 0 && eye_in_water);
    let empty = 10 - bubble_count(air + delay, max);
    Some(std::array::from_fn(|i| {
        let o = i as i32 + 1;
        if o <= full {
            Bubble::Full
        } else if full != popping && o == popping && eye_in_water {
            Bubble::Bursting
        } else if o > 10 - empty {
            Bubble::Empty
        } else {
            Bubble::Hidden
        }
    }))
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Heart {
    pub(crate) column: usize,
    pub(crate) row: usize,
    pub(crate) lift: f32,
    pub(crate) layers: Vec<&'static str>,
}

fn noise(tick: u64, index: usize) -> u64 {
    let mut x = tick.wrapping_mul(312_871) ^ (index as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15);
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51_afd7_ed55_8ccd);
    x ^ (x >> 33)
}

fn heart_sprite(v: &frame::McVitals, absorbing: bool, blinking: bool, half: bool) -> &'static str {
    let (poisoned, withered) = (v.poisoned, v.withered);
    if withered && !poisoned {
        return match (blinking, half) {
            (false, false) => "gui:hud/heart/withered_full",
            (false, true) => "gui:hud/heart/withered_half",
            (true, false) => "gui:hud/heart/withered_full_blinking",
            (true, true) => "gui:hud/heart/withered_half_blinking",
        };
    }
    if v.fully_frozen && !poisoned && !absorbing {
        return match (blinking, half) {
            (false, false) => "gui:hud/heart/frozen_full",
            (false, true) => "gui:hud/heart/frozen_half",
            (true, false) => "gui:hud/heart/frozen_full_blinking",
            (true, true) => "gui:hud/heart/frozen_half_blinking",
        };
    }
    match (absorbing, poisoned, blinking, half) {
        (true, _, false, false) => "gui:hud/heart/absorbing_full",
        (true, _, false, true) => "gui:hud/heart/absorbing_half",
        (true, _, true, false) => "gui:hud/heart/absorbing_full_blinking",
        (true, _, true, true) => "gui:hud/heart/absorbing_half_blinking",
        (false, true, false, false) => "gui:hud/heart/poisoned_full",
        (false, true, false, true) => "gui:hud/heart/poisoned_half",
        (false, true, true, false) => "gui:hud/heart/poisoned_full_blinking",
        (false, true, true, true) => "gui:hud/heart/poisoned_half_blinking",
        (false, false, false, false) => "gui:hud/heart/full",
        (false, false, false, true) => "gui:hud/heart/half",
        (false, false, true, false) => "gui:hud/heart/full_blinking",
        (false, false, true, true) => "gui:hud/heart/half_blinking",
    }
}

pub(crate) fn heart_rows(v: &frame::McVitals) -> (usize, f32) {
    let health = v.health.ceil();
    let max = v.max_health.max(health).max(v.display_health as f32);
    let rows = ((max + v.absorption.ceil()) / 2.0 / 10.0).ceil().max(1.0);
    (rows as usize, (10.0 - (rows - 2.0)).max(3.0))
}

pub(crate) fn hearts(v: &frame::McVitals) -> Vec<Heart> {
    let health = v.health.ceil() as i32;
    let max = v.max_health.max(health as f32).max(v.display_health as f32);
    let absorption = v.absorption.ceil() as i32;
    let containers = (max / 2.0).ceil() as i32;
    let extra = (absorption as f32 / 2.0).ceil() as i32;
    let low = health + absorption <= 4;
    let mut out = Vec::new();
    for l in (0..(containers + extra).max(0)).rev() {
        let mut lift = 0.0;
        if low {
            lift -= (noise(v.tick, l as usize) % 2) as f32;
        }
        if l < containers && l == v.regen_index {
            lift += 2.0;
        }
        let container = if v.blink {
            "gui:hud/heart/container_blinking"
        } else {
            "gui:hud/heart/container"
        };
        let mut layers = vec![container];
        let q = l * 2;
        if l >= containers {
            let r = q - containers * 2;
            if r < absorption {
                layers.push(heart_sprite(v, true, false, r + 1 == absorption));
            }
        }
        if v.blink && q < v.display_health {
            layers.push(heart_sprite(v, false, true, q + 1 == v.display_health));
        }
        if q < health {
            layers.push(heart_sprite(v, false, false, q + 1 == health));
        }
        out.push(Heart {
            column: (l % 10) as usize,
            row: (l / 10) as usize,
            lift,
            layers,
        });
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Drumstick {
    pub(crate) shake: f32,
    pub(crate) empty: &'static str,
    pub(crate) fill: Option<&'static str>,
}

pub(crate) fn drumsticks(v: &frame::McVitals) -> [Drumstick; 10] {
    let food = i32::from(v.food);
    let (empty, half, full) = if v.hunger {
        (
            "gui:hud/food_empty_hunger",
            "gui:hud/food_half_hunger",
            "gui:hud/food_full_hunger",
        )
    } else {
        (
            "gui:hud/food_empty",
            "gui:hud/food_half",
            "gui:hud/food_full",
        )
    };
    let shaking = v.saturation <= 0.0 && v.tick.is_multiple_of(food as u64 * 3 + 1);
    std::array::from_fn(|j| {
        let j2 = j as i32 * 2 + 1;
        Drumstick {
            shake: if shaking {
                (noise(v.tick, 100 + j) % 3) as f32 - 1.0
            } else {
                0.0
            },
            empty,
            fill: if j2 < food {
                Some(full)
            } else if j2 == food {
                Some(half)
            } else {
                None
            },
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vitals(health: f32, food: u8) -> frame::McVitals {
        frame::McVitals {
            health,
            max_health: 20.0,
            food,
            saturation: 5.0,
            display_health: health.ceil() as i32,
            regen_index: -1,
            ..frame::McVitals::default()
        }
    }

    fn fill(heart: &Heart) -> Option<&'static str> {
        heart.layers.get(1).copied()
    }

    #[test]
    fn wither_turns_the_hearts_black_unless_poison_turns_them_green() {
        let mut v = vitals(7.0, 20);
        v.withered = true;
        let black = hearts(&v);
        assert!(black.iter().any(|h| fill(h) == Some("gui:hud/heart/withered_full")));
        assert!(black.iter().any(|h| fill(h) == Some("gui:hud/heart/withered_half")));
        v.poisoned = true;
        assert!(hearts(&v).iter().all(|h| fill(h).is_none_or(|f| f.starts_with("gui:hud/heart/poisoned"))));
    }

    #[test]
    fn fully_frozen_players_get_frozen_hearts_but_poison_wins() {
        let mut v = vitals(7.0, 20);
        v.absorption = 2.0;
        v.frozen = 0.99;
        assert!(hearts(&v).iter().all(|h| h.layers.iter().all(|l| !l.contains("frozen"))));
        v.frozen = 1.0;
        v.fully_frozen = true;
        let frozen = hearts(&v);
        assert!(frozen.iter().any(|h| fill(h) == Some("gui:hud/heart/frozen_full")));
        assert!(frozen.iter().any(|h| fill(h) == Some("gui:hud/heart/frozen_half")));
        assert!(frozen.iter().any(|h| fill(h) == Some("gui:hud/heart/absorbing_full")));
        assert!(frozen.iter().all(|h| h.layers[0] == "gui:hud/heart/container"));
        v.blink = true;
        v.display_health = 9;
        assert!(hearts(&v).iter().any(|h| h.layers.contains(&"gui:hud/heart/frozen_full_blinking")));
        v.blink = false;
        v.withered = true;
        assert!(hearts(&v).iter().all(|h| fill(h).is_none_or(|f| !f.contains("frozen"))));
        v.poisoned = true;
        assert!(hearts(&v).iter().all(|h| fill(h).is_none_or(|f| f.starts_with("gui:hud/heart/poisoned") || f.starts_with("gui:hud/heart/absorbing"))));
    }

    #[test]
    fn twenty_health_is_ten_full_hearts() {
        let row = hearts(&vitals(20.0, 20));
        assert_eq!(row.len(), 10);
        assert!(row.iter().all(|h| fill(h) == Some("gui:hud/heart/full")));
        assert!(row.iter().all(|h| h.layers[0] == "gui:hud/heart/container"));
    }

    #[test]
    fn odd_health_ends_on_a_half_heart() {
        let row = hearts(&vitals(7.0, 20));
        let full = row
            .iter()
            .filter(|h| fill(h) == Some("gui:hud/heart/full"))
            .count();
        let half: Vec<_> = row
            .iter()
            .filter(|h| fill(h) == Some("gui:hud/heart/half"))
            .collect();
        assert_eq!(full, 3);
        assert_eq!(half.len(), 1);
        assert_eq!(half[0].column, 3);
        assert_eq!(row.iter().filter(|h| fill(h).is_none()).count(), 6);
    }

    #[test]
    fn absorption_adds_golden_hearts_on_a_second_row() {
        let mut v = vitals(20.0, 20);
        v.absorption = 4.0;
        let all = hearts(&v);
        assert_eq!(all.len(), 12);
        let golden: Vec<_> = all.iter().filter(|h| h.row == 1).collect();
        assert_eq!(golden.len(), 2);
        assert!(
            golden
                .iter()
                .all(|h| fill(h) == Some("gui:hud/heart/absorbing_full"))
        );
        assert_eq!(heart_rows(&v).0, 2);
    }

    #[test]
    fn poison_turns_the_hearts_green_and_low_health_shakes() {
        let mut v = vitals(3.0, 20);
        v.poisoned = true;
        let row = hearts(&v);
        assert!(
            row.iter()
                .any(|h| fill(h) == Some("gui:hud/heart/poisoned_half"))
        );
        let shakes = (0..50u64).any(|tick| {
            v.tick = tick;
            hearts(&v).iter().any(|h| h.lift != 0.0)
        });
        assert!(shakes);
    }

    #[test]
    fn regeneration_lifts_one_heart() {
        let mut v = vitals(20.0, 20);
        v.regen_index = 4;
        let lifted: Vec<_> = hearts(&v).into_iter().filter(|h| h.lift > 0.0).collect();
        assert_eq!(lifted.len(), 1);
        assert_eq!(lifted[0].column, 4);
    }

    #[test]
    fn food_fills_from_the_right_with_halves() {
        let row = drumsticks(&vitals(20.0, 7));
        assert_eq!(
            row.iter()
                .filter(|d| d.fill == Some("gui:hud/food_full"))
                .count(),
            3
        );
        assert_eq!(row[3].fill, Some("gui:hud/food_half"));
        assert_eq!(row[4].fill, None);
        assert_eq!(row[0].fill, Some("gui:hud/food_full"));
    }

    #[test]
    fn the_hunger_effect_greens_the_food_bar() {
        let mut v = vitals(20.0, 20);
        v.hunger = true;
        assert!(
            drumsticks(&v)
                .iter()
                .all(|d| d.empty == "gui:hud/food_empty_hunger")
        );
    }

    #[test]
    fn full_air_on_land_shows_nothing() {
        assert_eq!(bubbles(300, 300, false), None);
        assert_eq!(bubbles(300, 300, true), Some([Bubble::Full; 10]));
    }

    #[test]
    fn air_running_out_bursts_then_empties_bubbles() {
        let row = bubbles(271, 300, true).expect("bubbles under water");
        assert_eq!(&row[..9], &[Bubble::Full; 9]);
        assert_eq!(row[9], Bubble::Bursting);
        let row = bubbles(150, 300, true).expect("bubbles under water");
        assert_eq!(row.iter().filter(|b| **b == Bubble::Full).count(), 5);
        assert_eq!(row.iter().filter(|b| **b == Bubble::Empty).count(), 4);
        assert_eq!(bubbles(0, 300, true), Some([Bubble::Empty; 10]));
    }

    #[test]
    fn surfacing_refills_with_the_bubbles_showing() {
        let row = bubbles(120, 300, false).expect("refilling shows the row");
        assert_eq!(row.iter().filter(|b| **b == Bubble::Full).count(), 4);
        assert!(!row.contains(&Bubble::Bursting));
    }
}
