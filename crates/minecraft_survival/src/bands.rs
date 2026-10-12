use minecraft_net::Dimension;

pub const BLOCK: f64 = 36.0;
pub const WALL_BELOW: i32 = 128;
pub const WALL_ABOVE: i32 = 256;
pub const DIMENSIONS: [Dimension; 3] = [Dimension::Overworld, Dimension::Nether, Dimension::End];
const OVERWORLD_ORIGIN_Y: (f64, f64) = (32.0, 320.0);
const NETHER_ORIGIN_Y: f64 = 1314.0;
const END_ORIGIN_Y: f64 = -1140.0;
const NETHER_SCALE: f64 = 8.0;
const WALL_REACH: f64 = 0.05;
const BODY_HEIGHT: f64 = 1.8;

pub fn level(dimension: Dimension) -> u8 {
    match dimension {
        Dimension::Overworld => 0,
        Dimension::Nether => 1,
        Dimension::End => 2,
    }
}

pub fn band(dimension: Dimension) -> [f64; 2] {
    match dimension {
        Dimension::Overworld => [-20_000.0, 20_000.0],
        Dimension::Nether => [-60_000.0, -20_000.0],
        Dimension::End => [20_000.0, 60_000.0],
    }
}

pub fn heights(dimension: Dimension) -> (i32, i32) {
    match dimension {
        Dimension::Overworld => (-64, 384),
        Dimension::Nether | Dimension::End => (0, 256),
    }
}

pub fn wall_range(dimension: Dimension) -> [i32; 2] {
    let (min_y, height) = heights(dimension);
    [min_y - WALL_BELOW, min_y + height + WALL_ABOVE]
}

pub fn origin(dimension: Dimension, world_spawn: [f64; 3]) -> [f64; 3] {
    let [x, y, z] = world_spawn;
    match dimension {
        Dimension::Overworld => [x, y.clamp(OVERWORLD_ORIGIN_Y.0, OVERWORLD_ORIGIN_Y.1), z],
        Dimension::Nether => [x / NETHER_SCALE, NETHER_ORIGIN_Y, z / NETHER_SCALE],
        Dimension::End => [0.0, END_ORIGIN_Y, 0.0],
    }
}

pub fn origins(world_spawn: [f64; 3]) -> Vec<(Dimension, [f64; 3])> {
    DIMENSIONS
        .into_iter()
        .map(|dimension| (dimension, origin(dimension, world_spawn)))
        .collect()
}

pub fn at_bottom_wall(min_y: i32, feet_y: f64) -> bool {
    feet_y <= f64::from(min_y - WALL_BELOW) + WALL_REACH
}

pub fn kept_in_walls(dimension: Dimension, feet: [f64; 3]) -> [f64; 3] {
    let [bottom, top] = wall_range(dimension).map(f64::from);
    [feet[0], feet[1].clamp(bottom, top - BODY_HEIGHT), feet[2]]
}

pub fn check(world_spawn: [f64; 3]) -> Result<(), String> {
    for dimension in DIMENSIONS {
        let origin_y = origin(dimension, world_spawn)[1];
        let [lo, hi] = band(dimension);
        let walls = wall_range(dimension).map(|y| (f64::from(y) - origin_y) * BLOCK);
        if walls[0] < lo || walls[1] >= hi {
            return Err(format!(
                "Minecraft {} walls at map z {:?} leave its band {lo}..{hi}",
                dimension.dimension_type(),
                walls
            ));
        }
    }
    for (index, first) in DIMENSIONS.into_iter().enumerate() {
        for second in DIMENSIONS.into_iter().skip(index + 1) {
            let (a, b) = (band(first), band(second));
            if a[0] < b[1] && b[0] < a[1] {
                return Err(format!(
                    "Minecraft {} and {} bands overlap",
                    first.dimension_type(),
                    second.dimension_type()
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bands_hold_for_a_superflat_a_normal_and_the_highest_spawn() {
        for y in [-60.0, 64.0, 320.0] {
            assert_eq!(check([8.5, y, -24.5]), Ok(()), "spawn at y {y}");
        }
        assert_eq!(
            check([1.0e6, 64.0, -1.0e6]),
            Ok(()),
            "far spawns move only x and z"
        );
    }

    #[test]
    fn the_origins_are_fixed_by_the_world_spawn() {
        let spawn = [80.5, 70.0, -16.5];
        assert_eq!(origin(Dimension::Overworld, spawn), spawn);
        assert_eq!(
            origin(Dimension::Overworld, [80.5, -60.0, -16.5]),
            [80.5, 32.0, -16.5]
        );
        assert_eq!(origin(Dimension::Nether, spawn), [10.0625, 1314.0, -2.0625]);
        assert_eq!(origin(Dimension::End, spawn), [0.0, -1140.0, 0.0]);
        assert_eq!(
            origins(spawn),
            DIMENSIONS
                .into_iter()
                .map(|dimension| (dimension, origin(dimension, spawn)))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn the_design_numbers_for_every_wall() {
        assert_eq!(wall_range(Dimension::Overworld), [-192, 576]);
        assert_eq!(wall_range(Dimension::Nether), [-128, 512]);
        assert_eq!(wall_range(Dimension::End), [-128, 512]);
        let map_z =
            |dimension, y: i32| (f64::from(y) - origin(dimension, [0.0, 64.0, 0.0])[1]) * BLOCK;
        assert_eq!(map_z(Dimension::Nether, -128), -51_912.0);
        assert_eq!(map_z(Dimension::Nether, 512), -28_872.0);
        assert_eq!(map_z(Dimension::End, -128), 36_432.0);
        assert_eq!(map_z(Dimension::End, 512), 59_472.0);
        assert_eq!(
            DIMENSIONS.map(level),
            [0, 1, 2],
            "each dimension has its own level"
        );
    }

    #[test]
    fn a_teleport_past_a_wall_stops_at_it() {
        let inside = [0.5, 100.0, 0.5];
        assert_eq!(kept_in_walls(Dimension::Overworld, inside), inside);
        let high = kept_in_walls(Dimension::Overworld, [0.5, 700.0, 0.5]);
        assert!((high[1] - 574.2).abs() < 1e-9 && high[0] == 0.5 && high[2] == 0.5);
        assert_eq!(
            kept_in_walls(Dimension::End, [0.5, -400.0, 0.5]),
            [0.5, -128.0, 0.5]
        );
    }

    #[test]
    fn only_feet_on_the_bottom_wall_count_as_on_it() {
        assert!(at_bottom_wall(-64, -192.0));
        assert!(at_bottom_wall(-64, -192.0 + 0.125 / BLOCK));
        assert!(!at_bottom_wall(-64, -191.0));
        assert!(at_bottom_wall(0, -128.0));
        assert!(!at_bottom_wall(0, -64.0));
    }
}
