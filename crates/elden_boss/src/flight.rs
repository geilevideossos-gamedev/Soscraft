use glam::DVec3;

use crate::brain::{Dagger, Hit};
use crate::data::{BossDef, PlayerId, Weapon};
use crate::shape::{self, Body};

pub const THROWN: u8 = u8::MAX;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Flight {
    pub start: DVec3,
    pub velocity: DVec3,
    pub age: u16,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Flown {
    Flying,
    Struck(Hit),
    Stopped(DVec3),
    Spent,
}

impl Flight {
    pub fn launch(dagger: &Dagger) -> Flight {
        Flight {
            start: dagger.from,
            velocity: dagger.velocity,
            age: 0,
        }
    }

    pub fn at(&self, ticks: f64) -> DVec3 {
        self.start + self.velocity * ticks
    }

    pub fn path(&self) -> (DVec3, DVec3) {
        let from = self.at(f64::from(self.age));
        (from, from + self.velocity)
    }

    pub fn fly(&mut self, def: &BossDef, wall: Option<f64>, players: &[(PlayerId, Body)]) -> Flown {
        let (from, to) = self.path();
        let first = players
            .iter()
            .filter_map(|(player, body)| {
                shape::segment(from, to, body).map(|share| (share, *player))
            })
            .filter(|(share, _)| wall.is_none_or(|wall| *share <= wall))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((share, player)) = first {
            let daggers = def.daggers;
            let behind = from.lerp(to, share) - self.velocity.normalize_or_zero();
            return Flown::Struck(Hit {
                player,
                damage: daggers.damage,
                harm: daggers.harm,
                reaction: daggers.reaction,
                shield_off: 0,
                weapon: Some(Weapon::Dagger),
                from: behind,
                strike: THROWN,
            });
        }
        if let Some(share) = wall {
            return Flown::Stopped(from.lerp(to, share));
        }
        self.age += 1;
        if self.age >= def.daggers.ticks {
            Flown::Spent
        } else {
            Flown::Flying
        }
    }
}

pub fn share_of(from: DVec3, to: DVec3, at: DVec3) -> f64 {
    let length = from.distance(to);
    if length <= 0.0 {
        return 0.0;
    }
    (from.distance(at) / length).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Harm;
    use crate::margit::MARGIT;

    fn thrown() -> Flight {
        Flight::launch(&Dagger {
            from: DVec3::new(0.0, 1.0, 0.0),
            velocity: DVec3::new(0.0, 0.0, 1.5),
            target: None,
        })
    }

    fn standing(id: PlayerId, z: f64) -> (PlayerId, Body) {
        (id, Body::player(DVec3::new(0.0, 0.0, z), 1.62))
    }

    #[test]
    fn a_flight_stops_where_the_wall_or_the_nearer_player_is() {
        let mut flight = thrown();
        let mut moves = 0;
        let spent = loop {
            match flight.fly(&MARGIT, None, &[]) {
                Flown::Flying => moves += 1,
                other => break other,
            }
        };
        assert_eq!((spent, moves + 1), (Flown::Spent, 40));
        assert!((flight.at(f64::from(flight.age)).z - 60.0).abs() < 1e-9);
        let mut flight = thrown();
        flight.age = 2;
        let players = [standing(1, 5.0), standing(2, 3.9)];
        let Flown::Struck(hit) = flight.fly(&MARGIT, None, &players) else {
            panic!("the nearer player is struck");
        };
        assert_eq!((hit.player, hit.damage, hit.harm), (2, 4.0, Harm::Holy));
        assert_eq!((hit.weapon, hit.strike), (Some(Weapon::Dagger), THROWN));
        assert!((hit.from - DVec3::new(0.0, 1.0, 2.6)).length() < 1e-9);
        assert_eq!(flight.age, 2, "a dagger that hits is done");
        let Flown::Stopped(stopped) = flight.fly(&MARGIT, Some(0.2), &players) else {
            panic!("the wall is nearer than the player");
        };
        assert!((stopped - DVec3::new(0.0, 1.0, 3.3)).length() < 1e-9);
        let behind = flight.fly(&MARGIT, Some(0.9), &players);
        assert!(matches!(behind, Flown::Struck(hit) if hit.player == 2));
        assert_eq!(share_of(DVec3::ZERO, DVec3::Z * 2.0, DVec3::Z), 0.5);
    }
}
