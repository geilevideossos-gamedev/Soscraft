use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::Pos;

pub const TYPE_ID: &str = "minecraft:wandering_trader";
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.95;
pub const EYE_HEIGHT: f32 = 1.62;
pub const MAX_HEALTH: f32 = 20.0;
pub const MOVEMENT_SPEED: f64 = 0.699_999_988_079_071;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const STROLL_SPEED: f64 = 0.35;
pub const FLEE_SPEED: f64 = 0.5;
pub const WANDER_STOP: f64 = 2.0;
pub const WANDER_STEP: f64 = 10.0;
pub const DESPAWN_DELAY: i32 = 48000;
pub const HOME_RADIUS: i32 = 16;
pub const DRINK_TICKS: i32 = 32;
pub const INVISIBILITY_TICKS: i32 = 3600;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const TRADE_RANGE_SQR: f64 = 16.0;
pub const TRADE_SETS: [&str; 3] = [
    "minecraft:wandering_trader/buying",
    "minecraft:wandering_trader/uncommon",
    "minecraft:wandering_trader/common",
];
pub const AMBIENT_SOUND: &str = "entity.wandering_trader.ambient";
pub const TRADE_SOUND: &str = "entity.wandering_trader.trade";
pub const HURT_SOUND: &str = "entity.wandering_trader.hurt";
pub const DEATH_SOUND: &str = "entity.wandering_trader.death";
pub const YES_SOUND: &str = "entity.wandering_trader.yes";
pub const NO_SOUND: &str = "entity.wandering_trader.no";
pub const DRINK_POTION_SOUND: &str = "entity.wandering_trader.drink_potion";
pub const DRINK_MILK_SOUND: &str = "entity.wandering_trader.drink_milk";
pub const DISAPPEARED_SOUND: &str = "entity.wandering_trader.disappeared";
pub const REAPPEARED_SOUND: &str = "entity.wandering_trader.reappeared";
pub const POTION_ITEM: &str = "minecraft:potion";
pub const MILK_ITEM: &str = "minecraft:milk_bucket";
pub const INVISIBILITY_POTION: &str = "minecraft:invisibility";
pub const AVOIDED: [(&str, f64); 6] = [
    ("minecraft:zombie", 8.0),
    ("minecraft:husk", 8.0),
    ("minecraft:drowned", 8.0),
    ("minecraft:zombie_villager", 8.0),
    ("minecraft:zombified_piglin", 8.0),
    ("minecraft:zoglin", 10.0),
];
pub const ILLAGERS_AVOIDED: [(&str, f64); 5] = [
    ("minecraft:evoker", 12.0),
    ("minecraft:vindicator", 8.0),
    ("minecraft:vex", 8.0),
    ("minecraft:pillager", 15.0),
    ("minecraft:illusioner", 12.0),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drink {
    Invisibility,
    Milk,
}

impl Drink {
    pub fn item(self) -> &'static str {
        match self {
            Self::Invisibility => POTION_ITEM,
            Self::Milk => MILK_ITEM,
        }
    }

    pub fn components(self) -> Option<serde_json::Value> {
        match self {
            Self::Invisibility => Some(serde_json::json!({
                "minecraft:potion_contents": {"potion": INVISIBILITY_POTION}
            })),
            Self::Milk => None,
        }
    }

    pub fn drinking_sound(self) -> &'static str {
        match self {
            Self::Invisibility => DRINK_POTION_SOUND,
            Self::Milk => DRINK_MILK_SOUND,
        }
    }

    pub fn finish_sound(self) -> &'static str {
        match self {
            Self::Invisibility => DISAPPEARED_SOUND,
            Self::Milk => REAPPEARED_SOUND,
        }
    }
}

#[derive(Clone, Debug)]
pub struct WanderingTrader {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub despawn_delay: i32,
    pub wander_target: Option<Pos>,
    pub home: Option<(Pos, i32)>,
    pub drinking: Option<(Drink, i32)>,
    pub held: Option<Drink>,
    pub age: i32,
}

impl WanderingTrader {
    pub fn new(position: DVec3) -> Self {
        Self {
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            despawn_delay: 0,
            wander_target: None,
            home: None,
            drinking: None,
            held: None,
            age: 0,
        }
    }

    pub fn eye_height(&self) -> f32 {
        EYE_HEIGHT
    }

    pub fn within_home(&self) -> bool {
        let Some((home, radius)) = self.home else {
            return true;
        };
        let p = self.body.position;
        let block = (p.x.floor() as i32, p.y.floor() as i32, p.z.floor() as i32);
        let d = (
            i64::from(block.0 - home.0),
            i64::from(block.1 - home.1),
            i64::from(block.2 - home.2),
        );
        d.0 * d.0 + d.1 * d.1 + d.2 * d.2 < i64::from(radius) * i64::from(radius)
    }

    pub fn tick_despawn(&mut self, trading: bool) -> bool {
        if self.despawn_delay > 0 && !trading {
            self.despawn_delay -= 1;
            return self.despawn_delay == 0;
        }
        false
    }

    pub fn start_drinking(&mut self, drink: Drink) {
        self.held = Some(drink);
        self.drinking = Some((drink, DRINK_TICKS));
    }

    pub fn tick_drinking(&mut self) -> (Option<&'static str>, Option<Drink>) {
        let Some((drink, left)) = self.drinking else {
            return (None, None);
        };
        let left = left - 1;
        let sound = (left <= 25 && left % 4 == 0).then(|| drink.drinking_sound());
        if left <= 0 {
            self.drinking = None;
            return (sound, Some(drink));
        }
        self.drinking = Some((drink, left));
        (sound, None)
    }
}

pub fn closer_to_center_than(pos: Pos, position: DVec3, distance: f64) -> bool {
    let center = DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + 0.5,
        f64::from(pos.2) + 0.5,
    );
    center.distance_squared(position) < distance * distance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_trader_counts_its_stay_down_only_while_nobody_trades() {
        let mut trader = WanderingTrader::new(DVec3::ZERO);
        assert!(!trader.tick_despawn(false), "no delay, no leaving");
        trader.despawn_delay = 2;
        assert!(!trader.tick_despawn(true));
        assert_eq!(trader.despawn_delay, 2);
        assert!(!trader.tick_despawn(false));
        assert!(trader.tick_despawn(false));
        assert!(!trader.tick_despawn(false));
    }

    #[test]
    fn drinking_takes_32_ticks_with_a_gulp_every_fourth_from_25() {
        let mut trader = WanderingTrader::new(DVec3::ZERO);
        trader.start_drinking(Drink::Milk);
        let mut gulps = 0;
        let mut done = None;
        for _ in 0..DRINK_TICKS {
            let (sound, finished) = trader.tick_drinking();
            gulps += usize::from(sound.is_some());
            done = done.or(finished);
        }
        assert_eq!(done, Some(Drink::Milk));
        assert_eq!(gulps, 7);
        assert_eq!(trader.held, Some(Drink::Milk), "the goal empties the hand");
        assert!(trader.drinking.is_none());
    }

    #[test]
    fn home_and_wander_target_use_block_distances() {
        let mut trader = WanderingTrader::new(DVec3::new(10.5, 64.0, 0.5));
        assert!(trader.within_home());
        trader.home = Some(((0, 64, 0), 16));
        assert!(trader.within_home());
        trader.body.position = DVec3::new(16.5, 64.0, 0.5);
        assert!(!trader.within_home());
        assert!(closer_to_center_than(
            (0, 64, 0),
            DVec3::new(1.5, 64.0, 0.5),
            2.0
        ));
        assert!(!closer_to_center_than(
            (0, 64, 0),
            DVec3::new(2.6, 64.0, 0.5),
            2.0
        ));
    }
}
