use crate::bucket::{Change, ItemUse, Sound};
use glam::DVec3;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Player, World};
use serde_json::Value;

pub const FIREWORK_ROCKET: &str = "minecraft:firework_rocket";
pub const LAUNCH_SOUND: &str = "minecraft:entity.firework_rocket.launch";
pub const LAUNCH_VOLUME: f32 = 3.0;
pub const PLACEMENT_OFFSET: f64 = 0.15;
const USE_REACH: f64 = 4.5;
const FIREWORKS: &str = "minecraft:fireworks";
const REMOVED_FIREWORKS: &str = "!minecraft:fireworks";
const DEFAULT_FLIGHT: i32 = 1;
const MAX_EXPLOSIONS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Shape {
    SmallBall,
    LargeBall,
    Star,
    Creeper,
    Burst,
}

impl Shape {
    pub fn of(name: &str) -> Option<Self> {
        match name {
            "small_ball" => Some(Self::SmallBall),
            "large_ball" => Some(Self::LargeBall),
            "star" => Some(Self::Star),
            "creeper" => Some(Self::Creeper),
            "burst" => Some(Self::Burst),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Explosion {
    pub shape: Shape,
    pub colors: Vec<i32>,
    pub fade_colors: Vec<i32>,
    pub trail: bool,
    pub twinkle: bool,
}

impl Explosion {
    pub fn burst(color: i32) -> Self {
        Self {
            shape: Shape::Burst,
            colors: vec![color],
            fade_colors: Vec::new(),
            trail: false,
            twinkle: false,
        }
    }

    pub fn from_json(value: &Value) -> Option<Self> {
        let shape = Shape::of(value.get("shape")?.as_str()?)?;
        let colors = |key: &str| -> Vec<i32> {
            value
                .get(key)
                .and_then(Value::as_array)
                .map_or_else(Vec::new, |list| {
                    list.iter()
                        .filter_map(Value::as_i64)
                        .map(|c| c as i32)
                        .collect()
                })
        };
        let flag = |key: &str| {
            value.get(key).is_some_and(|flag| {
                flag.as_bool()
                    .unwrap_or_else(|| flag.as_i64().is_some_and(|n| n as i8 != 0))
            })
        };
        Some(Self {
            shape,
            colors: colors("colors"),
            fade_colors: colors("fade_colors"),
            trail: flag("has_trail"),
            twinkle: flag("has_twinkle"),
        })
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fireworks {
    pub flight: i32,
    pub explosions: Vec<Explosion>,
}

pub fn fireworks(stack: &ItemStack) -> Fireworks {
    let explosions = stack
        .components
        .as_ref()
        .and_then(|patch| patch.get(FIREWORKS))
        .and_then(|fireworks| fireworks.get("explosions"))
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |list| {
            list.iter()
                .take(MAX_EXPLOSIONS)
                .filter_map(Explosion::from_json)
                .collect()
        });
    Fireworks {
        flight: flight_duration(stack),
        explosions,
    }
}

pub fn flight_duration(stack: &ItemStack) -> i32 {
    let patch = stack.components.as_ref().and_then(Value::as_object);
    if let Some(fireworks) = patch.and_then(|patch| patch.get(FIREWORKS)) {
        return fireworks
            .get("flight_duration")
            .and_then(Value::as_i64)
            .map_or(0, |flight| flight.clamp(0, 255) as i32);
    }
    if patch.is_some_and(|patch| patch.contains_key(REMOVED_FIREWORKS)) {
        return 0;
    }
    DEFAULT_FLIGHT
}

pub fn lifetime(flight: i32, mut random: impl FnMut() -> f32) -> i32 {
    let mut next_int = |bound: i32| ((random() * bound as f32) as i32).clamp(0, bound - 1);
    10 * (1 + flight) + next_int(6) + next_int(7)
}

pub fn boost_ticks(stack: &ItemStack, random: impl FnMut() -> f32) -> i32 {
    lifetime(flight_duration(stack), random) + 1
}

#[derive(Clone, Debug, PartialEq)]
pub struct RocketLaunch {
    pub at: DVec3,
    pub fireworks: Fireworks,
}

pub fn rocket_place(
    player: &Player,
    world: &impl World,
    held: &ItemStack,
    gliding: bool,
) -> Option<ItemUse> {
    if held.id != FIREWORK_ROCKET || gliding {
        return None;
    }
    let hit = player.target(world, USE_REACH)?;
    let (x, y, z) = hit.face.offset();
    let face = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
    Some(ItemUse {
        change: Change::LaunchRocket(RocketLaunch {
            at: hit.point + face * PLACEMENT_OFFSET,
            fireworks: fireworks(held),
        }),
        result: None,
        wear: 0,
        consumed: true,
        sounds: Vec::new(),
    })
}

pub fn rocket_use(held: &ItemStack, gliding: bool, random: impl FnMut() -> f32) -> Option<ItemUse> {
    if held.id != FIREWORK_ROCKET || !gliding {
        return None;
    }
    Some(ItemUse {
        change: Change::Rocket(boost_ticks(held, random)),
        result: None,
        wear: 0,
        consumed: true,
        sounds: vec![Sound {
            event: LAUNCH_SOUND,
            at: None,
            volume: LAUNCH_VOLUME,
            pitch: 1.0,
        }],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rocket(components: Option<Value>) -> ItemStack {
        let mut stack = ItemStack::new(FIREWORK_ROCKET, 64);
        stack.components = components;
        stack
    }

    #[test]
    fn a_rocket_flies_one_unless_its_fireworks_say_otherwise() {
        assert_eq!(flight_duration(&rocket(None)), 1);
        assert_eq!(
            flight_duration(&rocket(Some(
                json!({"minecraft:fireworks": {"flight_duration": 3}})
            ))),
            3
        );
        assert_eq!(
            flight_duration(&rocket(Some(
                json!({"minecraft:fireworks": {"explosions": []}})
            ))),
            0
        );
        assert_eq!(
            flight_duration(&rocket(Some(json!({"!minecraft:fireworks": {}})))),
            0
        );
    }

    #[test]
    fn a_rocket_carries_the_stars_of_its_fireworks() {
        assert_eq!(fireworks(&rocket(None)), Fireworks { flight: 1, explosions: Vec::new() });
        let starred = rocket(Some(json!({"minecraft:fireworks": {
            "flight_duration": 2,
            "explosions": [
                {"shape": "large_ball", "colors": [11743532], "has_trail": 1},
                {"shape": "star", "colors": [14602026, 4312372], "fade_colors": [2437522], "has_twinkle": true},
                {"shape": "square", "colors": [1]},
                {"shape": "creeper", "has_trail": 0, "has_twinkle": false}
            ]
        }})));
        let read = fireworks(&starred);
        assert_eq!(read.flight, 2);
        assert_eq!(
            read.explosions,
            vec![
                Explosion {
                    shape: Shape::LargeBall,
                    colors: vec![11743532],
                    fade_colors: Vec::new(),
                    trail: true,
                    twinkle: false
                },
                Explosion {
                    shape: Shape::Star,
                    colors: vec![14602026, 4312372],
                    fade_colors: vec![2437522],
                    trail: false,
                    twinkle: true
                },
                Explosion {
                    shape: Shape::Creeper,
                    colors: Vec::new(),
                    fade_colors: Vec::new(),
                    trail: false,
                    twinkle: false
                },
            ]
        );
        let removed = rocket(Some(json!({"!minecraft:fireworks": {}})));
        assert_eq!(fireworks(&removed), Fireworks::default());
        assert_eq!(
            Explosion::burst(2437522),
            Explosion {
                shape: Shape::Burst,
                colors: vec![2437522],
                fade_colors: Vec::new(),
                trail: false,
                twinkle: false
            }
        );
    }

    #[test]
    fn the_boost_lasts_vanillas_lifetime_and_one_tick_more() {
        assert_eq!(lifetime(1, || 0.0), 20);
        assert_eq!(lifetime(1, || 0.999_999), 31);
        assert_eq!(lifetime(3, || 0.5), 40 + 3 + 3);
        assert_eq!(boost_ticks(&rocket(None), || 0.0), 21);
        assert_eq!(boost_ticks(&rocket(None), || 0.999_999), 32);
    }

    #[test]
    fn a_rocket_boosts_only_a_gliding_player_and_is_spent() {
        assert_eq!(rocket_use(&rocket(None), false, || 0.5), None);
        assert_eq!(
            rocket_use(&ItemStack::new("minecraft:stick", 1), true, || 0.5),
            None
        );
        let used =
            rocket_use(&rocket(None), true, || 0.0).expect("a gliding player uses the rocket");
        assert_eq!(used.change, Change::Rocket(21));
        assert!(used.consumed && used.result.is_none() && used.wear == 0);
        assert_eq!(used.sounds.len(), 1);
        assert_eq!(
            (
                used.sounds[0].event,
                used.sounds[0].at,
                used.sounds[0].volume
            ),
            (LAUNCH_SOUND, None, 3.0)
        );
    }

    use minecraftoss_player::{Block, Pos};

    struct Blocks(Vec<Pos>);

    impl World for Blocks {
        fn block(&self, pos: Pos) -> Option<Block> {
            self.0
                .contains(&pos)
                .then(|| Block::new("minecraft:grass_block"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn launched(used: ItemUse) -> RocketLaunch {
        assert!(used.consumed && used.result.is_none() && used.wear == 0);
        assert!(
            used.sounds.is_empty(),
            "the launch sounds on its first tick"
        );
        match used.change {
            Change::LaunchRocket(launch) => launch,
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_rocket_clicked_on_a_block_launches_just_off_the_clicked_face() {
        let ground = Blocks(vec![(0, 0, 0)]);
        let mut looking_down = Player::new(DVec3::new(0.5, 1.0, 0.5));
        looking_down.pitch = 90.0;
        let starred = rocket(Some(json!({"minecraft:fireworks": {
            "flight_duration": 2,
            "explosions": [{"shape": "burst", "colors": [11743532]}]
        }})));
        let on_top = launched(
            rocket_place(&looking_down, &ground, &starred, false).expect("the grass is clicked"),
        );
        assert!(
            (on_top.at - DVec3::new(0.5, 1.15, 0.5)).length() < 1e-9,
            "{}",
            on_top.at
        );
        assert_eq!(on_top.fireworks, fireworks(&starred));
        assert_eq!(on_top.fireworks.flight, 2);
        let wall = Blocks(vec![(2, 2, 0)]);
        let mut looking_east = Player::new(DVec3::new(0.5, 1.0, 0.5));
        looking_east.yaw = -90.0;
        let on_side = launched(
            rocket_place(&looking_east, &wall, &rocket(None), false).expect("the wall is clicked"),
        );
        assert!(
            (on_side.at - DVec3::new(2.0 - PLACEMENT_OFFSET, 2.62, 0.5)).length() < 1e-6,
            "{}",
            on_side.at
        );
        assert_eq!(
            on_side.fireworks,
            Fireworks {
                flight: 1,
                explosions: Vec::new()
            }
        );
        assert_eq!(
            rocket_place(&looking_down, &ground, &rocket(None), true),
            None,
            "a gliding player boosts instead"
        );
        assert_eq!(
            rocket_place(&looking_down, &Blocks(Vec::new()), &rocket(None), false),
            None,
            "nothing in reach"
        );
        assert_eq!(
            rocket_place(
                &looking_down,
                &ground,
                &ItemStack::new("minecraft:stick", 1),
                false
            ),
            None
        );
    }
}
