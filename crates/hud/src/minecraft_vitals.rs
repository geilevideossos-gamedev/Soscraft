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

#[cfg(test)]
mod tests {
    use super::*;

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
