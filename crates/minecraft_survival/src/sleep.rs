use crate::respawn_block::Pos;

pub const ZOMBIFIED_PIGLIN: &str = "minecraft:zombified_piglin";
pub const DEEP_SLEEP_TICKS: i32 = 100;

const MONSTERS: [&str; 33] = [
    "minecraft:zombie",
    "minecraft:husk",
    "minecraft:drowned",
    "minecraft:zombie_villager",
    "minecraft:giant",
    "minecraft:skeleton",
    "minecraft:stray",
    "minecraft:wither_skeleton",
    "minecraft:bogged",
    "minecraft:parched",
    "minecraft:creeper",
    "minecraft:spider",
    "minecraft:cave_spider",
    "minecraft:enderman",
    "minecraft:endermite",
    "minecraft:silverfish",
    "minecraft:witch",
    "minecraft:pillager",
    "minecraft:vindicator",
    "minecraft:evoker",
    "minecraft:illusioner",
    "minecraft:ravager",
    "minecraft:blaze",
    "minecraft:guardian",
    "minecraft:elder_guardian",
    "minecraft:vex",
    "minecraft:zoglin",
    "minecraft:breeze",
    "minecraft:creaking",
    "minecraft:warden",
    "minecraft:wither",
    "minecraft:piglin",
    "minecraft:piglin_brute",
];

pub fn dark_outside(sky_light: f32, fixed_time: bool) -> bool {
    !fixed_time && (15.0 - sky_light) as i32 >= 4
}

pub fn monster_box((x, y, z): Pos) -> [f64; 6] {
    let (x, y, z) = (f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5);
    [x - 8.0, y - 5.0, z - 8.0, x + 8.0, y + 5.0, z + 8.0]
}

pub fn prevents_rest(kind: &str, angry_at_sleeper: bool) -> bool {
    if kind == ZOMBIFIED_PIGLIN {
        return angry_at_sleeper;
    }
    MONSTERS.contains(&kind)
}

pub fn intersects(a: [f64; 6], b: [f64; 6]) -> bool {
    (0..3).all(|i| a[i] < b[i + 3] && a[i + 3] > b[i])
}

pub fn monster_in<'a>(
    area: [f64; 6],
    mobs: impl IntoIterator<Item = (&'a str, bool, [f64; 6])>,
) -> bool {
    mobs.into_iter()
        .any(|(kind, angry, bounds)| intersects(area, bounds) && prevents_rest(kind, angry))
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SleepStatus {
    pub active: i32,
    pub sleeping: i32,
}

impl SleepStatus {
    pub fn update(&mut self, players: impl IntoIterator<Item = (bool, bool)>) -> bool {
        let before = *self;
        *self = Self::default();
        for (spectator, sleeping) in players {
            if spectator {
                continue;
            }
            self.active += 1;
            if sleeping {
                self.sleeping += 1;
            }
        }
        (before.sleeping > 0 || self.sleeping > 0) && before != *self
    }

    pub fn sleepers_needed(&self, percentage: i32) -> i32 {
        let share = (self.active * percentage) as f32 / 100.0;
        1.max(share.ceil() as i32)
    }

    pub fn enough_sleeping(&self, percentage: i32) -> bool {
        self.sleeping >= self.sleepers_needed(percentage)
    }

    pub fn enough_deep_sleeping(&self, percentage: i32, deep: i32) -> bool {
        deep >= self.sleepers_needed(percentage)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_terrain::environment::DimensionEnvironment;
    use minecraftoss_core::Registries;

    fn registries() -> Option<Registries> {
        let paths = minecraftoss_core::registries::DataPaths::discover().ok()?;
        Registries::load(&paths).ok()
    }

    #[test]
    fn the_sky_is_dark_at_light_eleven_or_less() {
        assert!(dark_outside(11.0, false));
        assert!(!dark_outside(11.01, false));
        assert!(!dark_outside(15.0, false));
        assert!(dark_outside(4.0, false));
        for light in [0.0, 4.0, 11.0, 15.0] {
            assert!(!dark_outside(light, true));
        }
    }

    #[test]
    fn the_overworld_night_runs_from_tick_12523_to_23477_in_clear_weather() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionEnvironment::load(&registries, "minecraft:overworld").unwrap();
        let dark = |ticks: i64| dark_outside(overworld.level_sky_light(ticks, 0.0, 0.0), false);
        let night: Vec<i64> = (0..24000).filter(|&ticks| dark(ticks)).collect();
        assert_eq!(
            (night.first(), night.last(), night.len()),
            (Some(&12523), Some(&23477), 23477 - 12523 + 1)
        );
        assert!(dark(12523 + 24000 * 3) && !dark(12522 + 24000 * 3));
        let nether = DimensionEnvironment::load(&registries, "minecraft:the_nether").unwrap();
        assert_eq!(nether.level_sky_light(18000, 0.0, 0.0), 4.0);
    }

    #[test]
    fn a_thunderstorm_darkens_noon_but_rain_alone_does_not() {
        let Some(registries) = registries() else {
            return;
        };
        let overworld = DimensionEnvironment::load(&registries, "minecraft:overworld").unwrap();
        let rain = overworld.level_sky_light(6000, 1.0, 0.0);
        assert!((rain - 11.5625).abs() < 1.0e-4, "{rain}");
        assert!(!dark_outside(rain, false));
        let thunder = overworld.level_sky_light(6000, 1.0, 1.0);
        assert!(thunder <= 9.2 + 1.0e-4, "{thunder}");
        assert!(dark_outside(thunder, false));
    }

    #[test]
    fn sleepers_needed_rounds_up_and_needs_at_least_one() {
        let status = |active| SleepStatus {
            active,
            sleeping: 0,
        };
        assert_eq!(status(0).sleepers_needed(100), 1);
        assert_eq!(status(1).sleepers_needed(100), 1);
        assert_eq!(status(4).sleepers_needed(50), 2);
        assert_eq!(status(3).sleepers_needed(50), 2);
        assert_eq!(status(4).sleepers_needed(0), 1);
        assert_eq!(status(2).sleepers_needed(101), 3);
        let all = SleepStatus {
            active: 2,
            sleeping: 2,
        };
        assert!(!all.enough_sleeping(101) && !all.enough_deep_sleeping(101, 2));
        assert!(all.enough_sleeping(100) && all.enough_deep_sleeping(100, 2));
        assert!(!all.enough_deep_sleeping(100, 1));
    }

    #[test]
    fn the_sleep_count_changes_only_when_someone_sleeps() {
        let mut status = SleepStatus::default();
        assert!(!status.update([(false, false), (false, false)]));
        assert_eq!(
            status,
            SleepStatus {
                active: 2,
                sleeping: 0
            }
        );
        assert!(!status.update([(false, false)]), "nobody sleeps");
        assert!(status.update([(false, true), (false, false)]));
        assert!(!status.update([(false, true), (false, false)]), "no change");
        assert!(
            !status.update([(false, true), (false, false), (true, true)]),
            "a spectator does not count"
        );
        assert!(status.update([(false, true), (false, false), (false, false)]));
        assert!(status.update([(false, false), (false, false), (false, false)]));
        assert_eq!(status.sleeping, 0);
    }

    #[test]
    fn monsters_count_in_an_eight_by_five_box_around_the_head() {
        let area = monster_box((0, 64, 0));
        assert_eq!(area, [-7.5, 59.0, -7.5, 8.5, 69.0, 8.5]);
        let mob = |dx: f64, dy: f64, half: f64, height: f64| {
            let x = 0.5 + dx;
            [
                x - half,
                64.0 + dy,
                0.5 - half,
                x + half,
                64.0 + dy + height,
                0.5 + half,
            ]
        };
        let zombie = |dx, dy| mob(dx, dy, 0.3, 1.95);
        let near = |kind: &'static str, angry, bounds| monster_in(area, [(kind, angry, bounds)]);
        assert!(near("minecraft:zombie", false, zombie(8.2, 0.0)));
        assert!(!near("minecraft:zombie", false, zombie(8.4, 0.0)));
        assert!(near("minecraft:zombie", false, zombie(-8.2, 0.0)));
        assert!(near("minecraft:zombie", false, zombie(0.0, 4.99)));
        assert!(!near("minecraft:zombie", false, zombie(0.0, 5.0)));
        assert!(near("minecraft:zombie", false, zombie(0.0, -6.9)));
        assert!(!near("minecraft:zombie", false, zombie(0.0, -7.0)));
        for kind in [
            "minecraft:slime",
            "minecraft:magma_cube",
            "minecraft:ghast",
            "minecraft:phantom",
            "minecraft:shulker",
            "minecraft:hoglin",
            "minecraft:strider",
            "minecraft:iron_golem",
            "minecraft:wolf",
        ] {
            assert!(!near(kind, true, zombie(1.0, 0.0)), "{kind}");
        }
        for kind in [
            "minecraft:skeleton",
            "minecraft:creeper",
            "minecraft:spider",
            "minecraft:enderman",
            "minecraft:witch",
            "minecraft:blaze",
            "minecraft:zoglin",
            "minecraft:piglin",
            "minecraft:piglin_brute",
            "minecraft:wither_skeleton",
        ] {
            assert!(near(kind, false, zombie(1.0, 0.0)), "{kind}");
        }
        assert!(!near(ZOMBIFIED_PIGLIN, false, zombie(1.0, 0.0)));
        assert!(near(ZOMBIFIED_PIGLIN, true, zombie(1.0, 0.0)));
        assert!(monster_in(
            area,
            [
                ("minecraft:slime", false, zombie(1.0, 0.0)),
                ("minecraft:husk", false, zombie(-3.0, 1.0)),
            ]
        ));
    }
}
