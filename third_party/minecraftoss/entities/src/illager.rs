use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::Pos;
use minecraftoss_player::inventory::ItemStack;

pub const PILLAGER: &str = "minecraft:pillager";
pub const VINDICATOR: &str = "minecraft:vindicator";
pub const EVOKER: &str = "minecraft:evoker";
pub const RAIDERS: [&str; 6] = [
    PILLAGER,
    VINDICATOR,
    EVOKER,
    "minecraft:illusioner",
    "minecraft:witch",
    "minecraft:ravager",
];
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.95;
pub const EYE_HEIGHT: f32 = HEIGHT * 0.85;
pub const MAX_HEALTH: f32 = 24.0;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const EXPERIENCE: i32 = 5;
pub const DEFAULT_DROP_CHANCE: f32 = 0.085;
pub const INVENTORY_SIZE: usize = 5;
pub const CROSSBOW: &str = "minecraft:crossbow";
pub const CROSSBOW_DURABILITY: u32 = 465;
pub const CROSSBOW_CHARGE: i32 = 25;
pub const CROSSBOW_POWER: f32 = 1.6;
pub const CROSSBOW_RANGE: f32 = 8.0;
pub const CROSSBOW_SPEED: f64 = 1.0;
pub const LOADING_START: &str = "item.crossbow.loading_start";
pub const LOADING_MIDDLE: &str = "item.crossbow.loading_middle";
pub const LOADING_END: &str = "item.crossbow.loading_end";
pub const SHOOT_SOUND: &str = "item.crossbow.shoot";
pub const HIT_SOUND: &str = "item.crossbow.hit";
pub const HOLD_GROUND_RADIUS: f32 = 10.0;
pub const SHOUT_RANGE: f64 = 8.0;
pub const STROLL_SPEED: f64 = 0.6;
pub const LOOK_RANGE: f64 = 15.0;
pub const CREAKING_AVOID_RANGE: f64 = 8.0;
pub const CREAKING_WALK_SPEED: f64 = 1.0;
pub const CREAKING_SPRINT_SPEED: f64 = 1.2;
pub const PATROL_LEADER_CHANCE: f32 = 0.06;
pub const PATROL_SPEED: f64 = 0.7;
pub const PATROL_LEADER_SPEED: f64 = 0.595;
pub const PATROL_COMPANION_RANGE: f64 = 16.0;
pub const PATROL_ARRIVED: f64 = 10.0;
pub const PATROL_STEP: f64 = 10.0;
pub const PATROL_SWERVE: f64 = 0.4;
pub const PATROL_SWERVE_ANGLE: f32 = 90.0;
pub const PATROL_COOLDOWN: i64 = 200;
pub const PATROL_TARGET_SPREAD: i32 = 500;
pub const PATROL_WANDER: i32 = 8;
pub const PATROL_KEEP_RANGE: f64 = 128.0;
pub const IRON_AXE: &str = "minecraft:iron_axe";
pub const JOHNNY: &str = "Johnny";
pub const MELEE_SPEED: f64 = 1.0;
pub const VINDICATOR_PLAYER_LOOK_RANGE: f64 = 3.0;
pub const VINDICATOR_MOB_LOOK_RANGE: f64 = 8.0;
pub const ILLAGERS: [&str; 4] = [PILLAGER, VINDICATOR, EVOKER, "minecraft:illusioner"];
pub const PIERCING_ROLL: i32 = 300;

pub const MAINHAND: usize = 0;
pub const OFFHAND: usize = 1;
pub const HEAD: usize = 5;
pub const SLOT_NAMES: [&str; 6] = ["mainhand", "offhand", "feet", "legs", "chest", "head"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IllagerKind {
    Pillager,
    Vindicator,
    Evoker,
}

impl IllagerKind {
    pub fn type_id(self) -> &'static str {
        match self {
            Self::Pillager => PILLAGER,
            Self::Vindicator => VINDICATOR,
            Self::Evoker => EVOKER,
        }
    }

    pub fn from_type(id: &str) -> Option<Self> {
        match id {
            PILLAGER => Some(Self::Pillager),
            VINDICATOR => Some(Self::Vindicator),
            EVOKER => Some(Self::Evoker),
            _ => None,
        }
    }

    pub fn sound_family(self) -> &'static str {
        match self {
            Self::Pillager => "pillager",
            Self::Vindicator => "vindicator",
            Self::Evoker => "evoker",
        }
    }

    pub fn movement_speed(self) -> f64 {
        match self {
            Self::Pillager | Self::Vindicator => 0.349_999_994_039_535_5,
            Self::Evoker => 0.5,
        }
    }

    pub fn follow_range(self) -> f64 {
        match self {
            Self::Pillager => 32.0,
            Self::Vindicator | Self::Evoker => 12.0,
        }
    }

    pub fn attack_damage(self) -> f32 {
        match self {
            Self::Pillager | Self::Vindicator => 5.0,
            Self::Evoker => 2.0,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum IllagerArmPose {
    #[default]
    Crossed,
    Attacking,
    Spellcasting,
    BowAndArrow,
    CrossbowHold,
    CrossbowCharge,
    Celebrating,
    Neutral,
}

pub const ARM_POSES: [IllagerArmPose; 8] = [
    IllagerArmPose::Crossed,
    IllagerArmPose::Attacking,
    IllagerArmPose::Spellcasting,
    IllagerArmPose::BowAndArrow,
    IllagerArmPose::CrossbowHold,
    IllagerArmPose::CrossbowCharge,
    IllagerArmPose::Celebrating,
    IllagerArmPose::Neutral,
];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Raider {
    pub wave: i32,
    pub can_join_raid: bool,
    pub celebrating: bool,
    pub ticks_outside_raid: i32,
    pub raid_id: Option<i32>,
    pub patrol_target: Option<Pos>,
    pub patrol_leader: bool,
    pub patrolling: bool,
}

#[derive(Clone, Debug)]
pub struct Illager {
    pub kind: IllagerKind,
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub left_handed: bool,
    pub can_pick_up_loot: bool,
    pub equipment: [Option<ItemStack>; 6],
    pub drop_chances: [f32; 6],
    pub inventory: [Option<ItemStack>; INVENTORY_SIZE],
    pub raider: Raider,
    pub charging_crossbow: bool,
    pub aggressive: bool,
    pub johnny: bool,
}

impl Illager {
    pub fn new(kind: IllagerKind, position: DVec3) -> Self {
        Self {
            kind,
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            left_handed: false,
            can_pick_up_loot: kind == IllagerKind::Pillager,
            equipment: Default::default(),
            drop_chances: [DEFAULT_DROP_CHANCE; 6],
            inventory: Default::default(),
            raider: Raider::default(),
            charging_crossbow: false,
            aggressive: false,
            johnny: false,
        }
    }

    pub fn vindicator(position: DVec3) -> Self {
        let mut vindicator = Self::new(IllagerKind::Vindicator, position);
        vindicator.equipment[MAINHAND] = Some(iron_axe());
        vindicator
    }

    pub fn set_custom_name(&mut self, name: &str) {
        if self.kind == IllagerKind::Vindicator && name == JOHNNY {
            self.johnny = true;
        }
    }

    pub fn pillager(position: DVec3) -> Self {
        let mut pillager = Self::new(IllagerKind::Pillager, position);
        pillager.equipment[MAINHAND] = Some(crossbow());
        pillager
    }

    pub fn main_hand(&self) -> Option<&ItemStack> {
        self.equipment[MAINHAND].as_ref()
    }

    pub fn holds_crossbow(&self) -> bool {
        self.equipment[MAINHAND]
            .iter()
            .chain(&self.equipment[OFFHAND])
            .any(|stack| stack.id == CROSSBOW)
    }

    pub fn crossbow_hand(&self) -> usize {
        match &self.equipment[MAINHAND] {
            Some(stack) if stack.id == CROSSBOW => MAINHAND,
            _ => OFFHAND,
        }
    }

    pub fn crossbow_charged(&self) -> bool {
        self.equipment[self.crossbow_hand()]
            .as_ref()
            .is_some_and(|stack| {
                stack.id == CROSSBOW
                    && stack
                        .components
                        .as_ref()
                        .and_then(|c| c.get("minecraft:charged_projectiles"))
                        .and_then(|p| p.as_array())
                        .is_some_and(|p| !p.is_empty())
            })
    }

    pub fn set_crossbow_charged(&mut self, charged: bool) {
        let hand = self.crossbow_hand();
        let Some(stack) = self.equipment[hand]
            .as_mut()
            .filter(|stack| stack.id == CROSSBOW)
        else {
            return;
        };
        let mut components = match stack.components.take() {
            Some(serde_json::Value::Object(map)) => map,
            _ => serde_json::Map::new(),
        };
        if charged {
            components.insert(
                "minecraft:charged_projectiles".to_owned(),
                serde_json::json!([{"id": "minecraft:arrow", "count": 1}]),
            );
        } else {
            components.remove("minecraft:charged_projectiles");
        }
        stack.components =
            (!components.is_empty()).then_some(serde_json::Value::Object(components));
    }

    pub fn arm_pose(&self) -> IllagerArmPose {
        match self.kind {
            IllagerKind::Pillager if self.charging_crossbow => IllagerArmPose::CrossbowCharge,
            IllagerKind::Pillager if self.holds_crossbow() => IllagerArmPose::CrossbowHold,
            IllagerKind::Pillager if self.aggressive => IllagerArmPose::Attacking,
            IllagerKind::Pillager => IllagerArmPose::Neutral,
            IllagerKind::Vindicator if self.aggressive => IllagerArmPose::Attacking,
            IllagerKind::Vindicator if self.raider.celebrating => IllagerArmPose::Celebrating,
            IllagerKind::Vindicator | IllagerKind::Evoker => IllagerArmPose::Crossed,
        }
    }

    pub fn shows_held_items(&self) -> bool {
        self.kind != IllagerKind::Vindicator || self.aggressive
    }

    pub fn eye_height(&self) -> f32 {
        EYE_HEIGHT
    }

    pub fn is_patrolling(&self) -> bool {
        self.raider.patrolling
    }
}

pub fn crossbow() -> ItemStack {
    let mut stack = ItemStack::new(CROSSBOW, 1);
    stack.max = 1;
    stack
}

pub fn iron_axe() -> ItemStack {
    let mut stack = ItemStack::new(IRON_AXE, 1);
    stack.max = 1;
    stack
}

pub fn is_illager(kind: &str) -> bool {
    ILLAGERS.contains(&kind)
}

pub fn arm_pose_index(pose: IllagerArmPose) -> u8 {
    ARM_POSES.iter().position(|&p| p == pose).unwrap_or(0) as u8
}

pub fn arm_pose_of(index: u8) -> IllagerArmPose {
    ARM_POSES
        .get(usize::from(index))
        .copied()
        .unwrap_or_default()
}

pub fn is_raider(kind: &str) -> bool {
    RAIDERS.contains(&kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pillager_is_a_crossbow_raider_of_vanilla_size() {
        let pillager = Illager::pillager(DVec3::new(0.5, 64.0, 0.5));
        assert_eq!((pillager.body.width, pillager.body.height), (0.6, 1.95));
        assert!((pillager.eye_height() - 1.6575).abs() < 1e-5);
        assert_eq!(pillager.health, 24.0);
        assert_eq!(pillager.kind.follow_range(), 32.0);
        assert!(pillager.holds_crossbow() && !pillager.crossbow_charged());
        assert!(pillager.can_pick_up_loot);
        assert_eq!(pillager.arm_pose(), IllagerArmPose::CrossbowHold);
    }

    #[test]
    fn its_arm_pose_charges_holds_attacks_or_hangs() {
        let mut pillager = Illager::pillager(DVec3::ZERO);
        pillager.charging_crossbow = true;
        assert_eq!(pillager.arm_pose(), IllagerArmPose::CrossbowCharge);
        pillager.charging_crossbow = false;
        pillager.equipment[MAINHAND] = None;
        assert_eq!(pillager.arm_pose(), IllagerArmPose::Neutral);
        pillager.aggressive = true;
        assert_eq!(pillager.arm_pose(), IllagerArmPose::Attacking);
        for pose in ARM_POSES {
            assert_eq!(arm_pose_of(arm_pose_index(pose)), pose);
        }
    }

    #[test]
    fn a_charged_crossbow_carries_one_arrow_until_it_shoots() {
        let mut pillager = Illager::pillager(DVec3::ZERO);
        pillager.set_crossbow_charged(true);
        assert!(pillager.crossbow_charged());
        pillager.set_crossbow_charged(false);
        assert!(!pillager.crossbow_charged());
        assert!(pillager.main_hand().unwrap().components.is_none());
    }

    #[test]
    fn a_vindicator_holds_an_iron_axe_crosses_its_arms_and_raises_them_to_attack() {
        let mut vindicator = Illager::vindicator(DVec3::ZERO);
        assert_eq!(vindicator.kind.follow_range(), 12.0);
        assert_eq!(vindicator.kind.attack_damage(), 5.0);
        assert_eq!(vindicator.health, 24.0);
        assert_eq!(vindicator.main_hand().map(|s| s.id.as_str()), Some(IRON_AXE));
        assert!(!vindicator.can_pick_up_loot);
        assert_eq!(vindicator.arm_pose(), IllagerArmPose::Crossed);
        assert!(!vindicator.shows_held_items());
        vindicator.aggressive = true;
        assert_eq!(vindicator.arm_pose(), IllagerArmPose::Attacking);
        assert!(vindicator.shows_held_items());
        vindicator.aggressive = false;
        vindicator.raider.celebrating = true;
        assert_eq!(vindicator.arm_pose(), IllagerArmPose::Celebrating);
        assert!(Illager::pillager(DVec3::ZERO).shows_held_items());
    }

    #[test]
    fn only_a_vindicator_named_johnny_becomes_johnny() {
        let mut vindicator = Illager::vindicator(DVec3::ZERO);
        vindicator.set_custom_name("Bob");
        assert!(!vindicator.johnny);
        vindicator.set_custom_name(JOHNNY);
        assert!(vindicator.johnny);
        vindicator.set_custom_name("Bob");
        assert!(vindicator.johnny, "renaming keeps it Johnny");
        let mut pillager = Illager::pillager(DVec3::ZERO);
        pillager.set_custom_name(JOHNNY);
        assert!(!pillager.johnny);
        assert!(is_illager(VINDICATOR) && is_illager(PILLAGER) && !is_illager("minecraft:witch"));
    }

    #[test]
    fn raiders_are_the_illagers_the_witch_and_the_ravager() {
        assert!(is_raider(PILLAGER) && is_raider("minecraft:witch"));
        assert!(!is_raider("minecraft:zombie"));
        assert_eq!(
            IllagerKind::from_type(PILLAGER),
            Some(IllagerKind::Pillager)
        );
        assert_eq!(IllagerKind::from_type("minecraft:villager"), None);
    }
}
