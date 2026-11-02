use elden_boss::Flight;
use glam::DVec3;
use serde::{Deserialize, Serialize};

pub const DAGGERS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DaggerShow {
    pub start: DVec3,
    pub velocity: DVec3,
    pub age: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct EldenShow {
    pub daggers: [Option<DaggerShow>; DAGGERS],
}

impl EldenShow {
    pub fn of(daggers: &[Flight]) -> EldenShow {
        let newest = &daggers[daggers.len().saturating_sub(DAGGERS)..];
        let mut show = EldenShow::default();
        for (slot, flight) in show.daggers.iter_mut().zip(newest) {
            *slot = Some(DaggerShow {
                start: flight.start,
                velocity: flight.velocity,
                age: flight.age,
            });
        }
        show
    }

    pub fn flights(&self) -> Vec<Flight> {
        self.daggers
            .iter()
            .flatten()
            .map(|dagger| Flight {
                start: dagger.start,
                velocity: dagger.velocity,
                age: dagger.age,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flight(age: u16) -> Flight {
        Flight {
            start: DVec3::new(0.5, 66.2, f64::from(age)),
            velocity: DVec3::new(0.0, -0.05, 1.5),
            age,
        }
    }

    #[test]
    fn only_the_three_newest_daggers_are_sent_and_come_back_as_flights() {
        assert_eq!(EldenShow::of(&[]), EldenShow::default());
        assert!(EldenShow::default().flights().is_empty());
        let two = [flight(9), flight(1)];
        assert_eq!(EldenShow::of(&two).flights(), two);
        let four = [flight(30), flight(20), flight(10), flight(0)];
        let sent = EldenShow::of(&four);
        assert!(sent.daggers.iter().all(Option::is_some));
        assert_eq!(
            sent.flights(),
            four[1..],
            "the oldest is nearly spent and stays behind"
        );
    }
}
