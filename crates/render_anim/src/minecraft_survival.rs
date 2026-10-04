use minecraftoss_player::survival::SurvivalStatus;

pub(crate) const DROWN_DAMAGE: f32 = 2.0;

#[derive(Default)]
pub(crate) struct Survival {
    pub(crate) status: SurvivalStatus,
    pub(crate) eye_in_water: bool,
    bubbles: i32,
}

pub(crate) struct Breath {
    pub(crate) drowned: bool,
    pub(crate) popped: Option<i32>,
}

impl Survival {
    pub(crate) fn respawn(&mut self) {
        *self = Self::default();
    }

    pub(crate) fn breathe(&mut self, eye_in_water: bool) -> Breath {
        self.eye_in_water = eye_in_water;
        let drowned = self.status.tick_air(eye_in_water, false);
        let (air, max) = (
            self.status.air.clamp(0, self.status.max_air),
            self.status.max_air,
        );
        let bubbles = bubble_count(air - 2, max);
        let popped =
            (eye_in_water && bubbles < self.bubbles).then(|| 10 - bubble_count(air + 1, max));
        self.bubbles = bubbles;
        Breath { drowned, popped }
    }

    pub(crate) fn vitals(&self) -> frame::McVitals {
        frame::McVitals {
            air: self.status.air,
            max_air: self.status.max_air,
            eye_in_water: self.eye_in_water,
        }
    }
}

pub(crate) fn bubble_count(air: i32, max_air: i32) -> i32 {
    ((air * 10) as f32 / max_air.max(1) as f32).ceil() as i32
}

pub(crate) fn pop_sound(empty: i32) -> (f32, f32) {
    let volume = 0.5 + 0.1 * (empty - 2).max(0) as f32;
    let pitch = 1.0 + 0.1 * (empty - 4).max(0) as f32;
    (volume, pitch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn breath_runs_out_and_hurts_every_second_after_fifteen() {
        let mut survival = Survival::default();
        let mut drowned_at = Vec::new();
        for tick in 1..=320 {
            if survival.breathe(true).drowned {
                drowned_at.push(tick);
            }
        }
        assert_eq!(drowned_at, vec![320]);
        assert_eq!(survival.status.air, 0);
        for tick in 321..=340 {
            if survival.breathe(true).drowned {
                drowned_at.push(tick);
            }
        }
        assert_eq!(drowned_at, vec![320, 340]);
    }

    #[test]
    fn breath_comes_back_four_a_tick_above_water() {
        let mut survival = Survival::default();
        for _ in 0..100 {
            survival.breathe(true);
        }
        assert_eq!(survival.status.air, 200);
        survival.breathe(false);
        assert_eq!(survival.status.air, 204);
        for _ in 0..30 {
            survival.breathe(false);
        }
        assert_eq!(survival.status.air, 300);
    }

    #[test]
    fn bubbles_pop_one_by_one() {
        assert_eq!(bubble_count(298, 300), 10);
        assert_eq!(bubble_count(270, 300), 9);
        assert_eq!(bubble_count(0, 300), 0);
        assert_eq!(bubble_count(-2, 300), 0);
        let mut survival = Survival::default();
        let pops: Vec<i32> = (0..300)
            .filter_map(|_| survival.breathe(true).popped)
            .collect();
        assert_eq!(pops, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn later_pops_sound_louder_and_higher() {
        assert_eq!(pop_sound(0), (0.5, 1.0));
        let (volume, pitch) = pop_sound(9);
        assert!((volume - 1.2).abs() < 1e-6 && (pitch - 1.5).abs() < 1e-6);
    }
}
