pub const SUMMON_TARGET: (i32, i32, i32) = (0, 128, 0);
pub const GROWL_VOLUME: f32 = 64.0;
pub const PILLAR_BLAST: f32 = 5.0;
pub const CRYSTAL_BLAST: f32 = 6.0;
pub const SPIKE_CLEARANCE: i32 = 10;
pub const SPIKE_ORIGIN_Y: i32 = 45;
pub const PREPARING_TICKS: i32 = 100;
pub const TICKS_PER_PILLAR: i32 = 40;
pub const SUMMONING_TICKS: i32 = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stage {
    Start,
    PreparingToSummonPillars,
    SummoningPillars,
    SummoningDragon,
    End,
}

impl Stage {
    pub fn name(self) -> &'static str {
        match self {
            Stage::Start => "start",
            Stage::PreparingToSummonPillars => "preparing_to_summon_pillars",
            Stage::SummoningPillars => "summoning_pillars",
            Stage::SummoningDragon => "summoning_dragon",
            Stage::End => "end",
        }
    }

    pub fn by_name(name: &str) -> Option<Self> {
        [
            Stage::Start,
            Stage::PreparingToSummonPillars,
            Stage::SummoningPillars,
            Stage::SummoningDragon,
            Stage::End,
        ]
        .into_iter()
        .find(|stage| stage.name() == name)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Beam(Option<(i32, i32, i32)>),
    Growl,
    SpikeBeam(usize),
    RebuildSpike(usize),
    Summon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Respawn {
    pub stage: Stage,
    pub time: i32,
    pub crystals: Vec<[i32; 4]>,
}

impl Respawn {
    pub fn new(crystals: Vec<[i32; 4]>) -> Self {
        Self {
            stage: Stage::Start,
            time: 0,
            crystals,
        }
    }

    fn set_stage(&mut self, stage: Stage) {
        self.time = 0;
        self.stage = stage;
    }

    pub fn tick(&mut self, spikes: usize) -> Vec<Action> {
        let time = self.time;
        self.time += 1;
        let mut actions = Vec::new();
        match self.stage {
            Stage::Start => {
                actions.push(Action::Beam(Some(SUMMON_TARGET)));
                self.set_stage(Stage::PreparingToSummonPillars);
            }
            Stage::PreparingToSummonPillars => {
                if time >= PREPARING_TICKS {
                    self.set_stage(Stage::SummoningPillars);
                } else if time == 0 || (50..=52).contains(&time) || time >= 95 {
                    actions.push(Action::Growl);
                }
            }
            Stage::SummoningPillars => {
                let first = time % TICKS_PER_PILLAR == 0;
                let last = time % TICKS_PER_PILLAR == TICKS_PER_PILLAR - 1;
                let index = (time / TICKS_PER_PILLAR) as usize;
                if index < spikes {
                    if first {
                        actions.push(Action::SpikeBeam(index));
                    } else if last {
                        actions.push(Action::RebuildSpike(index));
                    }
                } else if first {
                    self.set_stage(Stage::SummoningDragon);
                }
            }
            Stage::SummoningDragon => {
                if time >= SUMMONING_TICKS {
                    self.set_stage(Stage::End);
                    actions.push(Action::Summon);
                } else if time >= 80 {
                    actions.push(Action::Growl);
                } else if time == 0 {
                    actions.push(Action::Beam(Some(SUMMON_TARGET)));
                } else if time < 5 {
                    actions.push(Action::Growl);
                }
            }
            Stage::End => {}
        }
        actions
    }
}

pub fn crystal_cells(exit_portal: (i32, i32, i32)) -> [(i32, i32, i32); 4] {
    let (x, y, z) = exit_portal;
    [
        (x, y + 1, z - 3),
        (x + 3, y + 1, z),
        (x, y + 1, z + 3),
        (x - 3, y + 1, z),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(respawn: &mut Respawn, spikes: usize) -> Vec<(usize, Stage, Action)> {
        let mut log = Vec::new();
        for tick in 0..2000 {
            let stage = respawn.stage;
            for action in respawn.tick(spikes) {
                log.push((tick, stage, action));
            }
            if respawn.stage == Stage::End {
                break;
            }
        }
        log
    }

    #[test]
    fn the_stages_keep_vanilla_names() {
        for stage in [
            Stage::Start,
            Stage::PreparingToSummonPillars,
            Stage::SummoningPillars,
            Stage::SummoningDragon,
            Stage::End,
        ] {
            assert_eq!(Stage::by_name(stage.name()), Some(stage));
        }
        assert_eq!(
            Stage::by_name("summoning_pillars"),
            Some(Stage::SummoningPillars)
        );
        assert_eq!(Stage::by_name("nope"), None);
    }

    #[test]
    fn the_respawn_beams_growls_rebuilds_each_pillar_and_summons_like_vanilla() {
        let mut respawn = Respawn::new(vec![[1, 2, 3, 4]]);
        let log = run(&mut respawn, 10);
        assert_eq!(log[0], (0, Stage::Start, Action::Beam(Some(SUMMON_TARGET))));
        let growls: Vec<usize> = log
            .iter()
            .filter(|(_, stage, action)| {
                *stage == Stage::PreparingToSummonPillars && *action == Action::Growl
            })
            .map(|(tick, _, _)| *tick - 1)
            .collect();
        assert_eq!(growls, [0, 50, 51, 52, 95, 96, 97, 98, 99]);
        let pillars: Vec<(usize, Action)> = log
            .iter()
            .filter(|(_, stage, _)| *stage == Stage::SummoningPillars)
            .map(|(tick, _, action)| (*tick, *action))
            .collect();
        assert_eq!(pillars.len(), 20);
        let start = pillars[0].0;
        assert_eq!(
            start, 102,
            "a tick to begin, 100 to prepare, one to move on"
        );
        for (index, pair) in pillars.chunks(2).enumerate() {
            assert_eq!(pair[0], (start + 40 * index, Action::SpikeBeam(index)));
            assert_eq!(
                pair[1],
                (start + 40 * index + 39, Action::RebuildSpike(index))
            );
        }
        let summoning: Vec<(usize, Action)> = log
            .iter()
            .filter(|(_, stage, _)| *stage == Stage::SummoningDragon)
            .map(|(tick, _, action)| (*tick, *action))
            .collect();
        let begin = start + 400 + 1;
        assert_eq!(summoning[0], (begin, Action::Beam(Some(SUMMON_TARGET))));
        assert_eq!(
            summoning[1..5]
                .iter()
                .map(|(tick, _)| tick - begin)
                .collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
        assert!(
            summoning[1..25]
                .iter()
                .all(|(_, action)| *action == Action::Growl)
        );
        assert_eq!(summoning.last(), Some(&(begin + 100, Action::Summon)));
        assert_eq!(respawn.stage, Stage::End);
        assert!(respawn.tick(10).is_empty(), "nothing after the end");
    }

    #[test]
    fn the_four_crystals_stand_three_blocks_out_one_above_the_portal() {
        assert_eq!(
            crystal_cells((0, 62, 0)),
            [(0, 63, -3), (3, 63, 0), (0, 63, 3), (-3, 63, 0)]
        );
    }
}
