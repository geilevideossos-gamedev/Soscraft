use minecraft_terrain::pack::PackStack;
use minecraft_terrain::terrain::Dimension;
use minecraftoss_player::rng::LegacyRandom;

const STARTING_DELAY: i32 = 100;
const DEFAULT_FREQUENCY: i32 = 20 * 1200;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Music {
    pub(crate) event: &'static str,
    pub(crate) min_delay: i32,
    pub(crate) max_delay: i32,
    pub(crate) replace: bool,
}

pub(crate) const CREDITS: Music = Music {
    event: "minecraft:music.credits",
    min_delay: 0,
    max_delay: 0,
    replace: true,
};

pub(crate) const END_BOSS: Music = Music {
    event: "minecraft:music.dragon",
    min_delay: 0,
    max_delay: 0,
    replace: true,
};

pub(crate) const END: Music = Music {
    event: "minecraft:music.end",
    min_delay: 6000,
    max_delay: 24000,
    replace: true,
};

pub(crate) fn situational(credits_open: bool, in_end: bool, boss_music: bool) -> Option<Music> {
    if credits_open {
        Some(CREDITS)
    } else if boss_music {
        Some(END_BOSS)
    } else if in_end {
        Some(END)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MusicAction {
    Start(Music),
    Stop,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct MusicInput<'a> {
    pub(crate) dimension: Dimension,
    pub(crate) credits_open: bool,
    pub(crate) boss_music: bool,
    pub(crate) finished: Option<&'a str>,
    pub(crate) loading: bool,
}

pub(crate) struct MusicManager {
    current: Option<&'static str>,
    delay: i32,
    dimension: Option<Dimension>,
    credits_open: bool,
    random: LegacyRandom,
}

impl Default for MusicManager {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |since| since.as_nanos() as u64);
        Self::new(seed)
    }
}

impl MusicManager {
    pub(crate) fn new(seed: u64) -> Self {
        Self {
            current: None,
            delay: STARTING_DELAY,
            dimension: None,
            credits_open: false,
            random: LegacyRandom::new(seed),
        }
    }

    fn next_int(&mut self, min: i32, max: i32) -> i32 {
        if min >= max {
            min
        } else {
            self.random.next_int((max - min + 1) as u32) as i32 + min
        }
    }

    fn next_song_delay(&mut self, music: Option<Music>) -> i32 {
        match music {
            None => DEFAULT_FREQUENCY,
            Some(music) => self.next_int(
                music.min_delay.min(DEFAULT_FREQUENCY),
                music.max_delay.min(DEFAULT_FREQUENCY),
            ),
        }
    }

    pub(crate) fn stop_playing(&mut self, situation: Option<Music>) -> Option<MusicAction> {
        let stopped = self.current.take().map(|_| MusicAction::Stop);
        self.delay = self.next_song_delay(situation) + STARTING_DELAY;
        stopped
    }

    pub(crate) fn tick(
        &mut self,
        situation: Option<Music>,
        active: bool,
        loading: bool,
    ) -> Option<MusicAction> {
        let Some(music) = situation else {
            self.delay = self.delay.max(STARTING_DELAY);
            return None;
        };
        let mut action = None;
        if let Some(current) = self.current {
            let mut active = active;
            if music.replace && music.event != current {
                action = Some(MusicAction::Stop);
                self.delay = self.next_int(0, music.min_delay / 2);
                active = false;
            }
            if !active {
                self.current = None;
                let next = self.next_song_delay(Some(music));
                self.delay = self.delay.min(next);
            }
        }
        self.delay = self.delay.min(music.max_delay);
        if self.current.is_none() && !loading {
            self.delay -= 1;
            if self.delay <= 0 {
                self.current = Some(music.event);
                self.delay = i32::MAX;
                action = Some(MusicAction::Start(music));
            }
        }
        action
    }

    pub(crate) fn update(&mut self, input: MusicInput) -> Option<MusicAction> {
        let situation = situational(
            input.credits_open,
            input.dimension == Dimension::End,
            input.boss_music,
        );
        let mut action = None;
        if self.credits_open && !input.credits_open && self.current == Some(CREDITS.event) {
            action = self.stop_playing(situation);
        }
        if situation.is_none() && self.current == Some(END_BOSS.event) {
            action = self.stop_playing(situation).or(action);
        }
        self.credits_open = input.credits_open;
        if self
            .dimension
            .is_some_and(|dimension| dimension != input.dimension)
        {
            action = self.stop_playing(situation).or(action);
        }
        self.dimension = Some(input.dimension);
        let active = self
            .current
            .is_some_and(|current| input.finished != Some(current));
        self.tick(situation, active, input.loading).or(action)
    }
}

pub(crate) fn apply(
    action: MusicAction,
    sounds: Option<&mut crate::minecraft_sounds::Sounds>,
    packs: &PackStack,
    music: &mut audio::McMusic,
) {
    match action {
        MusicAction::Stop => music.wanted = None,
        MusicAction::Start(start) => {
            music.finished = None;
            match sounds.and_then(|sounds| sounds.music_file(packs, start.event)) {
                Some((_, bytes, volume)) => {
                    music.wanted = Some(audio::McMusicTrack {
                        key: start.event.to_owned(),
                        bytes,
                        volume,
                    });
                }
                None => {
                    music.wanted = None;
                    music.finished = Some(start.event.to_owned());
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(dimension: Dimension, boss_music: bool, finished: Option<&str>) -> MusicInput<'_> {
        MusicInput {
            dimension,
            credits_open: false,
            boss_music,
            finished,
            loading: false,
        }
    }

    fn ticks_until_start(
        manager: &mut MusicManager,
        input: MusicInput,
        limit: usize,
    ) -> Option<(usize, Music)> {
        for tick in 1..=limit {
            if let Some(MusicAction::Start(music)) = manager.update(input) {
                return Some((tick, music));
            }
        }
        None
    }

    #[test]
    fn the_situation_is_the_credits_then_the_boss_then_the_end() {
        assert_eq!(situational(true, false, false), Some(CREDITS));
        assert_eq!(situational(true, true, true), Some(CREDITS));
        assert_eq!(situational(false, true, true), Some(END_BOSS));
        assert_eq!(situational(false, true, false), Some(END));
        assert_eq!(situational(false, false, true), Some(END_BOSS));
        assert_eq!(situational(false, false, false), None);
    }

    #[test]
    fn boss_music_plays_in_the_overworld_while_a_bar_asks_for_it_and_stops_after() {
        let mut manager = MusicManager::new(8);
        for _ in 0..200 {
            assert_eq!(
                manager.update(input(Dimension::Overworld, false, None)),
                None
            );
        }
        let fight = input(Dimension::Overworld, true, None);
        assert_eq!(
            manager.update(fight),
            Some(MusicAction::Start(END_BOSS)),
            "at once"
        );
        for _ in 0..500 {
            assert_eq!(manager.update(fight), None, "it plays on");
        }
        let ended = input(Dimension::Overworld, true, Some(END_BOSS.event));
        assert_eq!(
            manager.update(ended),
            Some(MusicAction::Start(END_BOSS)),
            "it loops while the bar asks"
        );
        assert_eq!(
            manager.update(input(Dimension::Overworld, false, None)),
            Some(MusicAction::Stop),
            "it stops when no bar asks any more"
        );
        for _ in 0..30000 {
            assert_eq!(
                manager.update(input(Dimension::Overworld, false, None)),
                None
            );
        }
        assert_eq!(
            manager.update(fight),
            Some(MusicAction::Start(END_BOSS)),
            "the next fight starts it again"
        );
        let mut nether = MusicManager::new(9);
        assert_eq!(
            nether.update(input(Dimension::Nether, true, None)),
            Some(MusicAction::Start(END_BOSS))
        );
        assert_eq!(
            nether.update(input(Dimension::Nether, false, None)),
            Some(MusicAction::Stop)
        );
    }

    #[test]
    fn the_end_music_rules_are_unchanged() {
        for credits in [false, true] {
            for boss in [false, true] {
                let expected = if credits {
                    CREDITS
                } else if boss {
                    END_BOSS
                } else {
                    END
                };
                assert_eq!(situational(credits, true, boss), Some(expected));
            }
        }
        for seed in 0..40 {
            let mut manager = MusicManager::new(seed);
            let fight = input(Dimension::End, true, None);
            assert_eq!(manager.update(fight), Some(MusicAction::Start(END_BOSS)));
            let after = manager.update(input(Dimension::End, false, None));
            assert!(
                matches!(
                    after,
                    Some(MusicAction::Stop) | Some(MusicAction::Start(END))
                ),
                "the End's own music takes over: {after:?}"
            );
        }
        let mut quiet = MusicManager::new(10);
        for _ in 0..5000 {
            assert_ne!(
                quiet.update(input(Dimension::End, false, None)),
                Some(MusicAction::Start(END_BOSS))
            );
        }
    }

    #[test]
    fn the_boss_music_starts_on_the_tick_its_bar_shows_and_loops() {
        let mut manager = MusicManager::new(1);
        let fight = input(Dimension::End, true, None);
        assert_eq!(manager.update(fight), Some(MusicAction::Start(END_BOSS)));
        for _ in 0..500 {
            assert_eq!(manager.update(fight), None, "it plays on");
        }
        let ended = input(Dimension::End, true, Some(END_BOSS.event));
        assert_eq!(
            manager.update(ended),
            Some(MusicAction::Start(END_BOSS)),
            "it starts again the tick it ends"
        );
        assert_eq!(manager.update(fight), None);
    }

    #[test]
    fn the_end_music_replaces_the_boss_music_within_3000_ticks_of_the_kill() {
        let mut waits = Vec::new();
        for seed in 0..40 {
            let mut manager = MusicManager::new(seed);
            manager.update(input(Dimension::End, true, None));
            let after = input(Dimension::End, false, None);
            let first = manager.update(after);
            let wait = match first {
                Some(MusicAction::Start(music)) => {
                    assert_eq!(music, END);
                    1
                }
                other => {
                    assert_eq!(
                        other,
                        Some(MusicAction::Stop),
                        "the boss music stops at once"
                    );
                    let (tick, music) = ticks_until_start(&mut manager, after, 4000).unwrap();
                    assert_eq!(music, END);
                    tick + 1
                }
            };
            assert!((1..=3001).contains(&wait), "{wait}");
            waits.push(wait);
        }
        assert!(waits.iter().any(|&wait| wait > 1500));
        assert!(waits.iter().any(|&wait| wait < 1500));
    }

    #[test]
    fn arriving_in_the_end_without_a_fight_waits_6100_to_24000_ticks() {
        let mut waits = Vec::new();
        for seed in 0..40 {
            let mut manager = MusicManager::new(seed);
            for _ in 0..200 {
                assert_eq!(
                    manager.update(input(Dimension::Overworld, false, None)),
                    None
                );
            }
            let (wait, music) =
                ticks_until_start(&mut manager, input(Dimension::End, false, None), 30000).unwrap();
            assert_eq!(music, END);
            assert!((6100..=24000).contains(&wait), "{wait}");
            waits.push(wait);
        }
        assert!(waits.iter().any(|&wait| wait < 12000));
        assert!(waits.iter().any(|&wait| wait > 18000));
    }

    #[test]
    fn the_credits_music_replaces_anything_and_stops_with_the_credits() {
        let mut manager = MusicManager::new(3);
        let quiet = input(Dimension::End, false, None);
        ticks_until_start(&mut manager, quiet, 200).unwrap();
        let credits = MusicInput {
            credits_open: true,
            ..quiet
        };
        assert_eq!(manager.update(credits), Some(MusicAction::Start(CREDITS)));
        for _ in 0..100 {
            assert_eq!(manager.update(credits), None);
        }
        assert_eq!(manager.update(quiet), Some(MusicAction::Stop));
        let (wait, music) = ticks_until_start(&mut manager, quiet, 30000).unwrap();
        assert_eq!(music, END);
        assert!((6100..=24000).contains(&(wait + 1)), "{wait}");
    }

    #[test]
    fn a_dimension_change_stops_the_music() {
        let mut manager = MusicManager::new(4);
        manager.update(input(Dimension::End, true, None));
        assert_eq!(
            manager.update(input(Dimension::Overworld, false, None)),
            Some(MusicAction::Stop)
        );
        for _ in 0..30000 {
            assert_eq!(
                manager.update(input(Dimension::Overworld, false, None)),
                None
            );
        }
    }

    #[test]
    fn nothing_counts_down_while_a_level_loads() {
        let mut manager = MusicManager::new(5);
        let loading = MusicInput {
            loading: true,
            ..input(Dimension::End, true, None)
        };
        for _ in 0..1000 {
            assert_eq!(manager.update(loading), None);
        }
        assert_eq!(
            manager.update(input(Dimension::End, true, None)),
            Some(MusicAction::Start(END_BOSS))
        );
        let mut fresh = MusicManager::new(5);
        let quiet = MusicInput {
            loading: true,
            ..input(Dimension::End, false, None)
        };
        for _ in 0..1000 {
            assert_eq!(fresh.update(quiet), None);
        }
        let (wait, _) = ticks_until_start(
            &mut fresh,
            MusicInput {
                loading: false,
                ..quiet
            },
            200,
        )
        .unwrap();
        assert_eq!(
            wait, STARTING_DELAY as usize,
            "the first 100 ticks wait for the load"
        );
    }

    #[test]
    fn outside_the_end_nothing_new_starts_but_the_current_track_plays_on() {
        let mut manager = MusicManager::new(6);
        let mut credits = MusicInput {
            credits_open: true,
            ..input(Dimension::Overworld, false, None)
        };
        assert_eq!(manager.update(credits), Some(MusicAction::Start(CREDITS)));
        credits.credits_open = false;
        assert_eq!(
            manager.update(credits),
            Some(MusicAction::Stop),
            "closing the credits stops their music"
        );
        let mut quiet = MusicManager::new(7);
        for _ in 0..50000 {
            assert_eq!(quiet.update(input(Dimension::Nether, false, None)), None);
        }
        assert_eq!(quiet.tick(Some(END), false, false), None);
        quiet.current = Some(END.event);
        assert_eq!(quiet.tick(None, true, false), None);
        assert_eq!(
            quiet.current,
            Some(END.event),
            "nothing replaces it outside the End"
        );
    }
}
