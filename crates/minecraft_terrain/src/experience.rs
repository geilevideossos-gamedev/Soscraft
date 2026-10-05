use crate::scene::Block;

const SPAWNER_BASE: u32 = 15;
const SPAWNER_ROLL_BOUND: u32 = 15;
const CREAKING_HEART_MIN: u32 = 20;
const CREAKING_HEART_MAX: u32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockExperience {
    Nothing,
    Constant(u32),
    Uniform(u32, u32),
    Spawner,
    CreakingHeart,
}

fn block_experience_kind(block: &Block) -> BlockExperience {
    if block.id.namespace != "minecraft" {
        return BlockExperience::Nothing;
    }
    match block.id.path.as_str() {
        "coal_ore" | "deepslate_coal_ore" => BlockExperience::Uniform(0, 2),
        "nether_gold_ore" => BlockExperience::Uniform(0, 1),
        "lapis_ore" | "deepslate_lapis_ore" | "nether_quartz_ore" => BlockExperience::Uniform(2, 5),
        "diamond_ore" | "deepslate_diamond_ore" | "emerald_ore" | "deepslate_emerald_ore" => {
            BlockExperience::Uniform(3, 7)
        }
        "redstone_ore" | "deepslate_redstone_ore" => BlockExperience::Uniform(1, 5),
        "sculk" => BlockExperience::Constant(1),
        "sculk_catalyst" | "sculk_sensor" | "calibrated_sculk_sensor" | "sculk_shrieker" => {
            BlockExperience::Constant(5)
        }
        "spawner" => BlockExperience::Spawner,
        "creaking_heart" => BlockExperience::CreakingHeart,
        _ => BlockExperience::Nothing,
    }
}

pub fn block_experience(
    block: &Block,
    by_player: bool,
    mut next_int: impl FnMut(u32) -> u32,
) -> u32 {
    let mut between = |min: u32, max: u32| min + next_int(max - min + 1);
    match block_experience_kind(block) {
        BlockExperience::Nothing => 0,
        BlockExperience::Constant(amount) => amount,
        BlockExperience::Uniform(min, max) => between(min, max),
        BlockExperience::Spawner => {
            SPAWNER_BASE + between(0, SPAWNER_ROLL_BOUND - 1) + between(0, SPAWNER_ROLL_BOUND - 1)
        }
        BlockExperience::CreakingHeart => {
            let natural = block
                .properties
                .get("natural")
                .is_some_and(|value| value == "true");
            if by_player && natural {
                between(CREAKING_HEART_MIN, CREAKING_HEART_MAX)
            } else {
                0
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::rng::LegacyRandom;

    fn rolls(id: &str, by_player: bool, count: usize) -> (Vec<u32>, usize) {
        let block = Block::new(id);
        let mut random = LegacyRandom::new(0x5eed);
        let mut draws = 0;
        let amounts = (0..count)
            .map(|_| {
                block_experience(&block, by_player, |bound| {
                    draws += 1;
                    random.next_int(bound)
                })
            })
            .collect();
        (amounts, draws)
    }

    fn assert_covers(id: &str, min: u32, max: u32) {
        let (amounts, draws) = rolls(id, true, 2000);
        assert_eq!(draws, 2000, "{id} draws once per break");
        assert!(
            amounts.iter().all(|&a| (min..=max).contains(&a)),
            "{id}: {amounts:?}"
        );
        for value in min..=max {
            assert!(amounts.contains(&value), "{id} never gave {value}");
        }
    }

    #[test]
    fn ores_give_their_uniform_ranges() {
        assert_covers("minecraft:coal_ore", 0, 2);
        assert_covers("minecraft:deepslate_coal_ore", 0, 2);
        assert_covers("minecraft:nether_gold_ore", 0, 1);
        assert_covers("minecraft:lapis_ore", 2, 5);
        assert_covers("minecraft:deepslate_lapis_ore", 2, 5);
        assert_covers("minecraft:nether_quartz_ore", 2, 5);
        assert_covers("minecraft:diamond_ore", 3, 7);
        assert_covers("minecraft:deepslate_diamond_ore", 3, 7);
        assert_covers("minecraft:emerald_ore", 3, 7);
        assert_covers("minecraft:deepslate_emerald_ore", 3, 7);
        assert_covers("minecraft:redstone_ore", 1, 5);
        assert_covers("minecraft:deepslate_redstone_ore", 1, 5);
    }

    #[test]
    fn uniform_draws_one_bounded_int_above_the_minimum() {
        let block = Block::new("minecraft:diamond_ore");
        let mut bounds = Vec::new();
        let lowest = block_experience(&block, true, |bound| {
            bounds.push(bound);
            0
        });
        let highest = block_experience(&block, true, |bound| bound - 1);
        assert_eq!((lowest, highest, bounds), (3, 7, vec![5]));
    }

    #[test]
    fn sculk_family_gives_constants_without_drawing() {
        for (id, amount) in [
            ("minecraft:sculk", 1),
            ("minecraft:sculk_catalyst", 5),
            ("minecraft:sculk_sensor", 5),
            ("minecraft:calibrated_sculk_sensor", 5),
            ("minecraft:sculk_shrieker", 5),
        ] {
            for by_player in [true, false] {
                let (amounts, draws) = rolls(id, by_player, 10);
                assert_eq!(draws, 0, "{id}");
                assert!(amounts.iter().all(|&a| a == amount), "{id}: {amounts:?}");
            }
        }
    }

    #[test]
    fn spawner_adds_two_rolls_of_fifteen_to_fifteen() {
        let (amounts, draws) = rolls("minecraft:spawner", false, 4000);
        assert_eq!(draws, 8000);
        assert!(
            amounts.iter().all(|&a| (15..=43).contains(&a)),
            "{amounts:?}"
        );
        assert!(amounts.contains(&15) && amounts.contains(&43));
        let mean = amounts.iter().sum::<u32>() as f64 / amounts.len() as f64;
        assert!((mean - 29.0).abs() < 0.5, "{mean}");
        let block = Block::new("minecraft:spawner");
        let mut bounds = Vec::new();
        let most = block_experience(&block, true, |bound| {
            bounds.push(bound);
            bound - 1
        });
        assert_eq!((most, bounds), (43, vec![15, 15]));
    }

    #[test]
    fn creaking_heart_needs_a_player_and_a_natural_heart() {
        let natural = Block::new("minecraft:creaking_heart").with("natural", "true");
        let placed = Block::new("minecraft:creaking_heart").with("natural", "false");
        let mut random = LegacyRandom::new(7);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..500 {
            seen.insert(block_experience(&natural, true, |bound| {
                random.next_int(bound)
            }));
        }
        assert_eq!(
            seen.into_iter().collect::<Vec<_>>(),
            vec![20, 21, 22, 23, 24]
        );
        let mut draws = 0;
        let mut count = |_: u32| {
            draws += 1;
            0
        };
        assert_eq!(block_experience(&natural, false, &mut count), 0);
        assert_eq!(block_experience(&placed, true, &mut count), 0);
        assert_eq!(draws, 0);
    }

    #[test]
    fn other_blocks_give_nothing() {
        for id in [
            "minecraft:iron_ore",
            "minecraft:deepslate_iron_ore",
            "minecraft:gold_ore",
            "minecraft:copper_ore",
            "minecraft:stone",
            "minecraft:sculk_vein",
            "minecraft:trial_spawner",
            "other:diamond_ore",
        ] {
            let (amounts, draws) = rolls(id, true, 10);
            assert_eq!(draws, 0, "{id}");
            assert!(amounts.iter().all(|&a| a == 0), "{id}");
        }
    }
}
