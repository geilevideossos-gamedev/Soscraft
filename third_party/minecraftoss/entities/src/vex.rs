use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::Pos;
use minecraftoss_player::inventory::ItemStack;

pub const TYPE_ID: &str = "minecraft:vex";
pub const WIDTH: f32 = 0.4;
pub const HEIGHT: f32 = 0.8;
pub const EYE_HEIGHT: f32 = 0.518_75;
pub const MAX_HEALTH: f32 = 14.0;
pub const ATTACK_DAMAGE: f32 = 4.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const EXPERIENCE: i32 = 3;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const LIFE_HURT_INTERVAL: i32 = 20;
pub const LIFE_HURT: f32 = 1.0;
pub const LIFE_BASE_SECONDS: i32 = 30;
pub const LIFE_SPREAD_SECONDS: u32 = 90;
pub const SUMMONED: usize = 3;
pub const SUMMON_SPREAD: u32 = 5;
pub const CHARGE_SOUND: &str = "entity.vex.charge";
pub const IRON_SWORD: &str = "minecraft:iron_sword";
pub const CHARGE_SPEED: f64 = 1.0;
pub const WANDER_SPEED: f64 = 0.25;
pub const CHARGE_START_SQR: f64 = 4.0;
pub const CHARGE_FOLLOW_SQR: f64 = 9.0;
pub const LOOK_RANGE: f64 = 3.0;
pub const MOB_LOOK_RANGE: f64 = 8.0;
pub const HORIZONTAL_DRAG: f64 = 0.91_f32 as f64;
pub const VERTICAL_DRAG: f64 = 0.98_f32 as f64;
pub const WAIT_BRAKE: f64 = 0.5;
pub const MAINHAND: usize = 0;
pub const OFFHAND: usize = 1;
pub const SLOT_NAMES: [&str; 6] = ["mainhand", "offhand", "feet", "legs", "chest", "head"];
pub const DEFAULT_DROP_CHANCE: f32 = 0.085;

#[derive(Clone, Debug)]
pub struct Vex {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub left_handed: bool,
    pub equipment: [Option<ItemStack>; 6],
    pub drop_chances: [f32; 6],
    pub owner: Option<u64>,
    pub owner_uuid: Option<u128>,
    pub bound_origin: Option<Pos>,
    pub life_ticks: Option<i32>,
    pub charging: bool,
}

pub fn iron_sword() -> ItemStack {
    let mut stack = ItemStack::new(IRON_SWORD, 1);
    stack.max = 1;
    stack
}

pub fn summoned_life(seconds: i32) -> i32 {
    20 * (LIFE_BASE_SECONDS + seconds)
}

impl Vex {
    pub fn new(position: DVec3) -> Self {
        let mut body = Body::new(position, WIDTH, HEIGHT);
        body.gravity = 0.0;
        Self {
            body,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            left_handed: false,
            equipment: Default::default(),
            drop_chances: [DEFAULT_DROP_CHANCE; 6],
            owner: None,
            owner_uuid: None,
            bound_origin: None,
            life_ticks: None,
            charging: false,
        }
    }

    pub fn armed(position: DVec3) -> Self {
        let mut vex = Self::new(position);
        vex.equipment[MAINHAND] = Some(iron_sword());
        vex.drop_chances[MAINHAND] = 0.0;
        vex
    }

    pub fn summoned(position: DVec3, owner: u64, origin: Pos, life: i32) -> Self {
        let mut vex = Self::armed(position);
        vex.owner = Some(owner);
        vex.bound_origin = Some(origin);
        vex.life_ticks = Some(life);
        vex
    }

    pub fn main_hand(&self) -> Option<&ItemStack> {
        self.equipment[MAINHAND].as_ref()
    }

    pub fn tick_life(&mut self) -> bool {
        let Some(ticks) = self.life_ticks.as_mut() else {
            return false;
        };
        *ticks -= 1;
        if *ticks > 0 {
            return false;
        }
        *ticks = LIFE_HURT_INTERVAL;
        true
    }

    pub fn size(&self) -> f64 {
        f64::from(self.body.width * 2.0 + self.body.height) / 3.0
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let p = self.body.position;
        let half = f64::from(self.body.width / 2.0);
        (
            DVec3::new(p.x - half, p.y, p.z - half),
            DVec3::new(p.x + half, p.y + f64::from(self.body.height), p.z + half),
        )
    }
}

pub fn yaw_towards(delta: DVec3) -> f32 {
    -(minecraftoss_player::mth::atan2(delta.x, delta.z) as f32) * 57.295_776
}

pub fn steer(
    velocity: DVec3,
    position: DVec3,
    wanted: DVec3,
    speed: f64,
    size: f64,
) -> Option<DVec3> {
    let delta = wanted - position;
    let distance = delta.length();
    if distance < size {
        return None;
    }
    Some(velocity + delta * (speed * 0.05 / distance))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vex_is_small_floats_and_carries_an_iron_sword_it_never_drops() {
        let vex = Vex::armed(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(
            (vex.body.width, vex.body.height, vex.health),
            (0.4, 0.8, 14.0)
        );
        assert_eq!(vex.body.gravity, 0.0);
        assert_eq!(vex.main_hand().map(|s| s.id.as_str()), Some(IRON_SWORD));
        assert_eq!(vex.drop_chances[MAINHAND], 0.0);
        assert!((vex.size() - 0.533_333).abs() < 1e-5);
    }

    #[test]
    fn a_summoned_vex_starts_hurting_itself_once_a_second_when_its_life_runs_out() {
        let mut vex = Vex::summoned(DVec3::ZERO, 7, (0, 64, 0), 3);
        assert_eq!(summoned_life(0), 600);
        assert_eq!(summoned_life(89), 2380);
        let hurts: Vec<bool> = (0..43).map(|_| vex.tick_life()).collect();
        let at: Vec<usize> = hurts
            .iter()
            .enumerate()
            .filter(|(_, h)| **h)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(at, vec![2, 22, 42]);
        let mut free = Vex::new(DVec3::ZERO);
        assert!((0..100).all(|_| !free.tick_life()));
    }

    #[test]
    fn it_steers_towards_where_it_wants_until_it_is_there() {
        let pushed = steer(
            DVec3::ZERO,
            DVec3::ZERO,
            DVec3::new(0.0, 0.0, 10.0),
            1.0,
            0.5,
        )
        .unwrap();
        assert!((pushed - DVec3::new(0.0, 0.0, 0.05)).length() < 1e-9);
        assert!(
            steer(
                DVec3::ZERO,
                DVec3::ZERO,
                DVec3::new(0.0, 0.3, 0.0),
                1.0,
                0.5
            )
            .is_none()
        );
        assert!((yaw_towards(DVec3::new(1.0, 0.0, 0.0)) + 90.0).abs() < 0.1);
        assert!(yaw_towards(DVec3::new(0.0, 0.0, 1.0)).abs() < 0.1);
    }
}
