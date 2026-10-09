use minecraft_net::{Dimension, PlayerUuid};
use minecraftoss_core::ChunkPos;

use crate::level::Level;
use crate::player::Player;

pub const FIGHT_ORIGIN: (i32, i32, i32) = (0, 0, 0);
pub const DRAGON_SPAWN_Y: i32 = 128;
pub const PLAYER_RANGE: f64 = 192.0;
pub const ARENA_TICKET_LEVEL: i32 = 9;
pub const TIME_BETWEEN_PLAYER_SCANS: u32 = 20;
pub const TICK_SECONDS: f64 = 0.05;

#[derive(Clone, Debug, PartialEq)]
pub struct DragonFight {
    pub clock: f64,
    pub ticks_since_last_player_scan: u32,
    pub players: Vec<PlayerUuid>,
}

impl Default for DragonFight {
    fn default() -> Self {
        Self {
            clock: 0.0,
            ticks_since_last_player_scan: TIME_BETWEEN_PLAYER_SCANS + 1,
            players: Vec::new(),
        }
    }
}

pub fn arena_ticket() -> (ChunkPos, i32) {
    let (x, _, z) = FIGHT_ORIGIN;
    (
        ChunkPos::new(x.div_euclid(16), z.div_euclid(16)),
        ARENA_TICKET_LEVEL - 1,
    )
}

pub fn valid_player(feet: [f64; 3], alive: bool) -> bool {
    let (x, y, z) = FIGHT_ORIGIN;
    let centre = [f64::from(x), f64::from(DRAGON_SPAWN_Y + y), f64::from(z)];
    let distance: f64 = feet
        .iter()
        .zip(centre)
        .map(|(a, b)| (a - b) * (a - b))
        .sum();
    alive && distance <= PLAYER_RANGE * PLAYER_RANGE
}

impl DragonFight {
    pub fn ticks(&mut self, seconds: f64) -> u32 {
        self.clock += seconds.max(0.0);
        let ticks = (self.clock / TICK_SECONDS).floor();
        self.clock -= ticks * TICK_SECONDS;
        ticks.min(f64::from(u32::MAX)) as u32
    }

    pub fn tick(&mut self, valid: impl Fn() -> Vec<PlayerUuid>) -> Vec<(ChunkPos, i32)> {
        self.ticks_since_last_player_scan += 1;
        if self.ticks_since_last_player_scan >= TIME_BETWEEN_PLAYER_SCANS {
            self.players = valid();
            self.ticks_since_last_player_scan = 0;
        }
        if self.players.is_empty() {
            Vec::new()
        } else {
            vec![arena_ticket()]
        }
    }
}

pub fn valid_players<'a>(
    players: impl Iterator<Item = &'a Player>,
    dimension: Dimension,
) -> Vec<PlayerUuid> {
    players
        .filter(|player| player.dimension == dimension && player.switching.is_none())
        .filter_map(|player| {
            let place = player.place.filter(|place| place.dimension == dimension)?;
            valid_player(place.feet, !player.dead).then(|| player.uuid.clone())
        })
        .collect()
}

pub fn tick_level<'a>(
    level: &mut Level,
    players: impl Iterator<Item = &'a Player> + Clone,
    seconds: f64,
) -> Option<usize> {
    if level.dimension != Dimension::End {
        return None;
    }
    let dimension = level.dimension;
    let fight = level.dragon_fight.get_or_insert_with(DragonFight::default);
    let ticks = fight.ticks(seconds);
    let before = fight.players.len();
    let mut tickets = None;
    for _ in 0..ticks {
        tickets = Some(fight.tick(|| valid_players(players.clone(), dimension)));
    }
    let after = fight.players.len();
    if let Some(tickets) = tickets {
        level.terrain.stream.set_tickets(&tickets);
    }
    (before.min(1) != after.min(1)).then_some(after)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uuid(name: &str) -> PlayerUuid {
        PlayerUuid(name.to_owned())
    }

    #[test]
    fn the_arena_ticket_keeps_chunks_minus_eight_to_eight_ticking() {
        assert_eq!(arena_ticket(), (ChunkPos::new(0, 0), 8));
    }

    #[test]
    fn players_count_within_192_blocks_of_the_dragon_spawn_while_alive() {
        assert!(valid_player([100.5, 49.0, 0.5], true));
        assert!(valid_player([0.0, 128.0, 192.0], true));
        assert!(!valid_player([0.0, 128.0, 192.01], true));
        assert!(!valid_player([150.0, 0.0, 0.0], true));
        assert!(!valid_player([0.0, 128.0, 0.0], false));
    }

    #[test]
    fn the_ticket_follows_the_players_scanned_every_twenty_ticks() {
        let mut fight = DragonFight::default();
        let steve = vec![uuid("steve")];
        assert_eq!(
            fight.tick(|| steve.clone()),
            vec![arena_ticket()],
            "the first tick scans"
        );
        assert_eq!(fight.players, steve);
        for _ in 0..19 {
            assert_eq!(
                fight.tick(Vec::new),
                vec![arena_ticket()],
                "players are kept between scans"
            );
        }
        assert_eq!(
            fight.tick(Vec::new),
            Vec::new(),
            "the twentieth tick scans again"
        );
        assert!(fight.players.is_empty());
    }

    #[test]
    fn the_fight_clock_counts_whole_ticks() {
        let mut fight = DragonFight::default();
        assert_eq!(fight.ticks(0.12), 2);
        assert_eq!(fight.ticks(0.04), 1);
    }

    fn stream_until(level: &mut Level, at: (i32, i32, i32), done: impl Fn(&Level) -> bool) {
        let started = std::time::Instant::now();
        while !done(level) {
            assert!(
                started.elapsed().as_secs() < 600,
                "the stream never got there"
            );
            let Level {
                terrain,
                entities,
                containers,
                ..
            } = level;
            crate::level::stream_tick(terrain, entities, containers, at);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    #[test]
    fn the_arena_ticket_loads_the_arena_beside_the_view_and_lets_it_go() {
        let Some(paths) = minecraftoss_core::registries::DataPaths::discover().ok() else {
            return;
        };
        let Some(registries) = minecraftoss_core::Registries::load(&paths).ok() else {
            return;
        };
        let terrain = crate::level::Terrain::open(
            std::sync::Arc::new(registries),
            0,
            2,
            Dimension::End,
            None,
            None,
        )
        .expect("an End level");
        let mut level = Level::new(Dimension::End, terrain);
        let platform = (100, 49, 0);
        let has =
            |level: &Level, chunk: (i32, i32)| level.terrain.scene.generated_chunk(chunk).is_some();
        stream_until(&mut level, platform, |level| has(level, (6, 0)));
        assert!(!has(&level, (0, 0)), "the island centre is out of view");
        let mut steve = crate::the_end::tests::player(Dimension::End);
        steve.place = Some(crate::player_nbt::Place::standing(
            Dimension::End,
            [100.5, 49.0, 0.5],
            90.0,
            0.0,
        ));
        assert_eq!(tick_level(&mut level, [&steve].into_iter(), 0.05), Some(1));
        assert_eq!(tick_level(&mut level, [&steve].into_iter(), 0.05), None);
        let corners = [(-8, -8), (8, -8), (-8, 8), (8, 8), (0, 0)];
        stream_until(&mut level, platform, |level| {
            corners.iter().all(|&chunk| has(level, chunk))
        });
        for chunk in [(-9, 0), (0, 9), (-9, -9)] {
            assert!(!has(&level, chunk), "{chunk:?} is outside the ticket");
        }
        steve.place = Some(crate::player_nbt::Place::standing(
            Dimension::End,
            [100.5, 49.0, 200.5],
            90.0,
            0.0,
        ));
        for _ in 0..18 {
            assert_eq!(
                tick_level(&mut level, [&steve].into_iter(), 0.05),
                None,
                "until the next scan"
            );
        }
        assert_eq!(tick_level(&mut level, [&steve].into_iter(), 0.05), Some(0));
        stream_until(&mut level, platform, |level| {
            corners.iter().all(|&chunk| !has(level, chunk))
        });
        assert!(has(&level, (6, 0)), "the view stays");
    }
}
