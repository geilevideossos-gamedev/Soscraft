use glam::DVec3;
use minecraftoss_core::random::RandomSource;

pub const CAT_TICK_DELAY: i32 = 1200;
pub const CAT_SPAWNS_IN: [&str; 1] = ["minecraft:swamp_hut"];
pub const VILLAGE_SECTIONS: i32 = 2;
pub const VILLAGE_HOMES: usize = 4;
pub const VILLAGE_CATS: usize = 5;
pub const VILLAGE_REACH: (f64, f64) = (48.0, 8.0);
pub const HUT_REACH: (f64, f64) = (16.0, 8.0);

#[derive(Clone, Debug, Default)]
pub struct CustomSpawners {
    pub cats: Option<CatSpawner>,
}

impl CustomSpawners {
    pub fn for_dimension(dimension_type: &str) -> Self {
        Self {
            cats: (dimension_type == "minecraft:overworld").then(CatSpawner::default),
        }
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
