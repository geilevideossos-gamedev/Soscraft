use glam::DVec3;
use minecraftoss_core::random::RandomSource;

pub const CAT_TICK_DELAY: i32 = 1200;
pub const CAT_SPAWNS_IN: [&str; 1] = ["minecraft:swamp_hut"];
pub const VILLAGE_SECTIONS: i32 = 2;
pub const VILLAGE_HOMES: usize = 4;
pub const VILLAGE_CATS: usize = 5;
pub const VILLAGE_REACH: (f64, f64) = (48.0, 8.0);
pub const HUT_REACH: (f64, f64) = (16.0, 8.0);
pub const TRADER_TICK_DELAY: i32 = 1200;
pub const TRADER_SPAWN_DELAY: i32 = 24000;
pub const TRADER_MIN_CHANCE: i32 = 25;
pub const TRADER_MAX_CHANCE: i32 = 75;
pub const TRADER_CHANCE_STEP: i32 = 25;
pub const TRADER_ONE_IN: i32 = 10;
pub const TRADER_RANGE: i32 = 48;
pub const LLAMA_RANGE: i32 = 4;
pub const TRADER_LLAMAS: usize = 2;
pub const TRADER_DATA: &str = "wandering_trader.dat";
pub const NO_TRADER_BIOMES: &str = "minecraft:without_wandering_trader_spawns";

#[derive(Clone, Debug)]
pub struct CustomSpawners {
    pub cats: Option<CatSpawner>,
    pub traders: Option<TraderSpawner>,
    pub spawn_wandering_traders: bool,
}

impl Default for CustomSpawners {
    fn default() -> Self {
        Self {
            cats: None,
            traders: None,
            spawn_wandering_traders: true,
        }
    }
}

impl CustomSpawners {
    pub fn for_dimension(dimension_type: &str) -> Self {
        let overworld = dimension_type == "minecraft:overworld";
        Self {
            cats: overworld.then(CatSpawner::default),
            traders: overworld.then(TraderSpawner::default),
            spawn_wandering_traders: true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TraderData {
    pub spawn_delay: i32,
    pub spawn_chance: i32,
}

impl Default for TraderData {
    fn default() -> Self {
        Self {
            spawn_delay: TRADER_SPAWN_DELAY,
            spawn_chance: TRADER_MIN_CHANCE,
        }
    }
}

impl TraderData {
    pub fn read(tag: &minecraftoss_core::nbt::Tag) -> Self {
        let int = |key: &str, default: i32| {
            tag.get(key)
                .and_then(minecraftoss_core::nbt::Tag::as_i64)
                .map_or(default, |v| v as i32)
        };
        Self {
            spawn_delay: int("spawn_delay", TRADER_SPAWN_DELAY),
            spawn_chance: int("spawn_chance", TRADER_MIN_CHANCE),
        }
    }

    pub fn write(&self) -> minecraftoss_core::nbt::Tag {
        use minecraftoss_core::nbt::Tag;
        Tag::Compound(
            [
                ("spawn_delay".to_owned(), Tag::Int(self.spawn_delay)),
                ("spawn_chance".to_owned(), Tag::Int(self.spawn_chance)),
            ]
            .into_iter()
            .collect(),
        )
    }
}

#[derive(Clone, Debug)]
pub struct TraderSpawner {
    pub tick_delay: i32,
    pub data: TraderData,
    pub random: minecraftoss_core::random::LegacyRandom,
}

impl Default for TraderSpawner {
    fn default() -> Self {
        let seed = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as i64);
        Self {
            tick_delay: TRADER_TICK_DELAY,
            data: TraderData::default(),
            random: minecraftoss_core::random::LegacyRandom::new(seed),
        }
    }
}

impl TraderSpawner {
    pub fn due(&mut self, rule: bool, spawn_mobs: bool) -> bool {
        if !rule {
            return false;
        }
        self.tick_delay -= 1;
        if self.tick_delay > 0 {
            return false;
        }
        self.tick_delay = TRADER_TICK_DELAY;
        self.data.spawn_delay -= TRADER_TICK_DELAY;
        if self.data.spawn_delay > 0 {
            return false;
        }
        self.data.spawn_delay = TRADER_SPAWN_DELAY;
        if !spawn_mobs {
            return false;
        }
        let chance = self.data.spawn_chance;
        self.data.spawn_chance =
            (chance + TRADER_CHANCE_STEP).clamp(TRADER_MIN_CHANCE, TRADER_MAX_CHANCE);
        self.random.next_i32_bound(100) <= chance
    }

    pub fn spawned(&mut self) {
        self.data.spawn_chance = TRADER_MIN_CHANCE;
    }

    pub fn tries(&mut self) -> bool {
        self.random.next_i32_bound(TRADER_ONE_IN) == 0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CatSpawner {
    pub next_tick: i32,
}

impl CatSpawner {
    pub fn next_spot(
        &mut self,
        random: &mut dyn RandomSource,
        players: &[DVec3],
    ) -> Option<(i32, i32, i32)> {
        self.next_tick -= 1;
        if self.next_tick > 0 {
            return None;
        }
        self.next_tick = CAT_TICK_DELAY;
        if players.is_empty() {
            return None;
        }
        let player = players[random.next_i32_bound(players.len() as i32) as usize];
        let x = (8 + random.next_i32_bound(24)) * if random.next_bool() { -1 } else { 1 };
        let z = (8 + random.next_i32_bound(24)) * if random.next_bool() { -1 } else { 1 };
        let feet = (
            player.x.floor() as i32,
            player.y.floor() as i32,
            player.z.floor() as i32,
        );
        Some((feet.0 + x, feet.1, feet.2 + z))
    }
}

pub fn chunks_around(pos: (i32, i32, i32)) -> Vec<(i32, i32)> {
    let mut out = Vec::new();
    for cx in (pos.0 - 10) >> 4..=(pos.0 + 10) >> 4 {
        for cz in (pos.2 - 10) >> 4..=(pos.2 + 10) >> 4 {
            out.push((cx, cz));
        }
    }
    out
}

pub fn reach_box(pos: (i32, i32, i32), reach: (f64, f64)) -> (DVec3, DVec3) {
    let low = DVec3::new(f64::from(pos.0), f64::from(pos.1), f64::from(pos.2));
    let size = DVec3::new(reach.0, reach.1, reach.0);
    (low - size, low + DVec3::ONE + size)
}

pub fn village_wants_cat(occupied_homes: usize, cats: usize) -> bool {
    occupied_homes > VILLAGE_HOMES && cats < VILLAGE_CATS
}

pub fn hut_wants_cat(cats: usize) -> bool {
    cats == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::random::LegacyRandom;

    #[test]
    fn the_cat_spawner_looks_every_minute_between_eight_and_thirty_one_blocks_away() {
        let mut spawner = CatSpawner::default();
        let mut random = LegacyRandom::new(7);
        let players = [DVec3::new(100.5, 70.0, -40.5)];
        let first = spawner
            .next_spot(&mut random, &players)
            .expect("it looks at once");
        assert_eq!(spawner.next_tick, CAT_TICK_DELAY);
        for _ in 0..CAT_TICK_DELAY - 1 {
            assert_eq!(spawner.next_spot(&mut random, &players), None);
        }
        let second = spawner
            .next_spot(&mut random, &players)
            .expect("and again a minute later");
        for spot in [first, second] {
            let (dx, dz) = ((spot.0 - 100).abs(), (spot.2 + 41).abs());
            assert!((8..=31).contains(&dx) && (8..=31).contains(&dz), "{spot:?}");
            assert_eq!(spot.1, 70);
        }
        let mut alone = CatSpawner::default();
        assert_eq!(alone.next_spot(&mut random, &[]), None);
        assert_eq!(
            alone.next_tick, CAT_TICK_DELAY,
            "no player still waits a minute"
        );
    }

    #[test]
    fn the_trader_spawner_rolls_every_twenty_minutes_at_25_then_50_then_75_percent() {
        let mut spawner = TraderSpawner {
            random: LegacyRandom::new(11),
            ..TraderSpawner::default()
        };
        let mut rolls = 0;
        for tick in 1..=TRADER_SPAWN_DELAY {
            if spawner.due(true, true) {
                rolls += 1;
            }
            if tick % TRADER_TICK_DELAY == 0 && tick < TRADER_SPAWN_DELAY {
                assert_eq!(spawner.data.spawn_chance, TRADER_MIN_CHANCE, "{tick}");
            }
        }
        assert!(rolls <= 1);
        assert_eq!(spawner.data.spawn_delay, TRADER_SPAWN_DELAY);
        assert_eq!(spawner.data.spawn_chance, 50);
        let mut chances = Vec::new();
        for _ in 0..3 {
            spawner.data.spawn_delay = 1;
            spawner.tick_delay = 1;
            spawner.due(true, true);
            chances.push(spawner.data.spawn_chance);
        }
        assert_eq!(chances, [75, 75, 75]);
        spawner.spawned();
        assert_eq!(spawner.data.spawn_chance, 25);
        let stopped = spawner.data;
        for _ in 0..TRADER_SPAWN_DELAY * 2 {
            assert!(!spawner.due(false, true));
        }
        assert_eq!(spawner.data, stopped, "the rule off stops the clock");
        spawner.data.spawn_delay = 1;
        spawner.tick_delay = 1;
        assert!(!spawner.due(true, false));
        assert_eq!(
            spawner.data,
            TraderData {
                spawn_delay: TRADER_SPAWN_DELAY,
                spawn_chance: 25
            },
            "without mob spawning the day passes with no roll"
        );
        let mut hits = [0_i32; 3];
        for (slot, chance) in [25, 50, 75].into_iter().enumerate() {
            for _ in 0..4000 {
                spawner.data = TraderData {
                    spawn_delay: 1,
                    spawn_chance: chance,
                };
                spawner.tick_delay = 1;
                if spawner.due(true, true) {
                    hits[slot] += 1;
                }
            }
        }
        for (hit, chance) in hits.into_iter().zip([26, 51, 76]) {
            assert!((hit * 100 / 4000 - chance).abs() <= 3, "{hits:?}");
        }
        let tries = (0..10000).filter(|_| spawner.tries()).count();
        assert!((900..1100).contains(&tries), "{tries}");
    }

    #[test]
    fn the_trader_data_reads_and_writes_the_vanilla_fields() {
        let data = TraderData {
            spawn_delay: 4800,
            spawn_chance: 75,
        };
        let tag = data.write();
        assert_eq!(
            tag.get("spawn_delay"),
            Some(&minecraftoss_core::nbt::Tag::Int(4800))
        );
        assert_eq!(TraderData::read(&tag), data);
        assert_eq!(
            TraderData::read(&minecraftoss_core::nbt::Tag::Compound(Default::default())),
            TraderData::default()
        );
        assert!(CustomSpawners::for_dimension("minecraft:overworld")
            .traders
            .is_some());
        assert!(CustomSpawners::for_dimension("minecraft:the_nether")
            .traders
            .is_none());
    }

    #[test]
    fn villages_take_up_to_five_cats_with_more_than_four_homes_and_huts_one() {
        assert!(!village_wants_cat(4, 0));
        assert!(village_wants_cat(5, 4));
        assert!(!village_wants_cat(9, 5));
        assert!(hut_wants_cat(0) && !hut_wants_cat(1));
        let (min, max) = reach_box((0, 64, 0), VILLAGE_REACH);
        assert_eq!(
            (min, max),
            (DVec3::new(-48.0, 56.0, -48.0), DVec3::new(49.0, 73.0, 49.0))
        );
        assert_eq!(
            chunks_around((5, 64, 5)),
            vec![(-1, -1), (-1, 0), (0, -1), (0, 0)]
        );
        assert_eq!(
            CustomSpawners::for_dimension("minecraft:overworld").cats,
            Some(CatSpawner::default())
        );
        assert!(
            CustomSpawners::for_dimension("minecraft:the_nether")
                .cats
                .is_none()
        );
    }
}
