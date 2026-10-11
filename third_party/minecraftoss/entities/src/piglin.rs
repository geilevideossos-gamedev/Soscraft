use crate::health::{DamageResult, DamageState};
use crate::movement::Body;
use glam::DVec3;
use minecraftoss_player::inventory::ItemStack;

pub const MAX_HEALTH: f32 = 16.0;
pub const BRUTE_MAX_HEALTH: f32 = 50.0;
pub const MOVEMENT_SPEED: f64 = 0.3499999940395355;
pub const BABY_SPEED_BONUS: f64 = 0.20000000298023224;
pub const ATTACK_DAMAGE: f32 = 5.0;
pub const BRUTE_ATTACK_DAMAGE: f32 = 7.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const BRUTE_FOLLOW_RANGE: f64 = 12.0;
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 1.95;
pub const EYE_HEIGHT: f32 = 1.79;
pub const BABY_WIDTH: f32 = 0.49;
pub const BABY_HEIGHT: f32 = 0.98;
pub const BABY_EYE_HEIGHT: f32 = 0.78;
pub const EXPERIENCE: i32 = 5;
pub const BRUTE_EXPERIENCE: i32 = 20;
pub const CONVERSION_TIME: i32 = 300;
pub const AMBIENT_INTERVAL: i32 = 80;
pub const STEP_VOLUME: f32 = 0.15;
pub const DEFAULT_DROP_CHANCE: f32 = 0.085;
pub const INVENTORY_SIZE: usize = 8;

pub const MAINHAND: usize = 0;
pub const OFFHAND: usize = 1;
pub const FEET: usize = 2;
pub const LEGS: usize = 3;
pub const CHEST: usize = 4;
pub const HEAD: usize = 5;
pub const SLOT_NAMES: [&str; 6] = ["mainhand", "offhand", "feet", "legs", "chest", "head"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PiglinKind {
    Piglin,
    Brute,
}

impl PiglinKind {
    pub fn type_id(self) -> &'static str {
        match self {
            Self::Piglin => "minecraft:piglin",
            Self::Brute => "minecraft:piglin_brute",
        }
    }

    pub fn family(self) -> &'static str {
        match self {
            Self::Piglin => "piglin",
            Self::Brute => "piglin_brute",
        }
    }

    pub fn from_type(id: &str) -> Option<Self> {
        match id {
            "minecraft:piglin" => Some(Self::Piglin),
            "minecraft:piglin_brute" => Some(Self::Brute),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ArmPose {
    #[default]
    Default,
    Dancing,
    AdmiringItem,
    AttackingWithMeleeWeapon,
    CrossbowCharge,
    CrossbowHold,
}

#[derive(Clone, Debug)]
pub struct Piglin {
    pub kind: PiglinKind,
    pub body: Body,
    pub yaw: f32,
    pub health: f32,
    pub damage: DamageState,
    pub baby: bool,
    pub armor: [Option<ItemStack>; 4],
    pub main_hand: Option<ItemStack>,
    pub offhand: Option<ItemStack>,
    pub inventory: [Option<ItemStack>; INVENTORY_SIZE],
    pub drop_chances: [f32; 6],
    pub cannot_hunt: bool,
    pub immune_to_zombification: bool,
    pub time_in_overworld: i32,
    pub persistence_required: bool,
    pub can_pick_up_loot: bool,
    pub left_handed: bool,
    pub charging_crossbow: bool,
    pub dancing: bool,
    pub aggressive: bool,
    pub converting: bool,
}

impl Piglin {
    pub fn new(kind: PiglinKind, position: DVec3) -> Self {
        Self {
            kind,
            body: Body::new(position, WIDTH, HEIGHT),
            yaw: 0.0,
            health: max_health(kind),
            damage: DamageState::default(),
            baby: false,
            armor: Default::default(),
            main_hand: None,
            offhand: None,
            inventory: Default::default(),
            drop_chances: [DEFAULT_DROP_CHANCE; 6],
            cannot_hunt: false,
            immune_to_zombification: false,
            time_in_overworld: 0,
            persistence_required: false,
            can_pick_up_loot: true,
            left_handed: false,
            charging_crossbow: false,
            dancing: false,
            aggressive: false,
            converting: false,
        }
    }

    pub fn is_brute(&self) -> bool {
        self.kind == PiglinKind::Brute
    }

    pub fn is_baby(&self) -> bool {
        self.baby && !self.is_brute()
    }

    pub fn set_baby(&mut self, baby: bool) {
        self.baby = baby && !self.is_brute();
        self.sync_dimensions();
    }

    pub fn sync_dimensions(&mut self) {
        let (width, height) = if self.is_baby() {
            (BABY_WIDTH, BABY_HEIGHT)
        } else {
            (WIDTH, HEIGHT)
        };
        self.body.width = width;
        self.body.height = height;
    }

    pub fn eye_height(&self) -> f32 {
        if self.is_baby() {
            BABY_EYE_HEIGHT
        } else {
            EYE_HEIGHT
        }
    }

    pub fn max_health(&self) -> f32 {
        max_health(self.kind)
    }

    pub fn movement_speed(&self) -> f64 {
        if self.is_baby() {
            MOVEMENT_SPEED + MOVEMENT_SPEED * BABY_SPEED_BONUS
        } else {
            MOVEMENT_SPEED
        }
    }

    pub fn attack_damage(&self) -> f32 {
        match self.kind {
            PiglinKind::Piglin => ATTACK_DAMAGE,
            PiglinKind::Brute => BRUTE_ATTACK_DAMAGE,
        }
    }

    pub fn follow_range(&self) -> f64 {
        match self.kind {
            PiglinKind::Piglin => FOLLOW_RANGE,
            PiglinKind::Brute => BRUTE_FOLLOW_RANGE,
        }
    }

    pub fn equipment(&self, slot: usize) -> Option<&ItemStack> {
        match slot {
            MAINHAND => self.main_hand.as_ref(),
            OFFHAND => self.offhand.as_ref(),
            FEET..=HEAD => self.armor[HEAD - slot].as_ref(),
            _ => None,
        }
    }

    pub fn equipment_mut(&mut self, slot: usize) -> Option<&mut Option<ItemStack>> {
        match slot {
            MAINHAND => Some(&mut self.main_hand),
            OFFHAND => Some(&mut self.offhand),
            FEET..=HEAD => Some(&mut self.armor[HEAD - slot]),
            _ => None,
        }
    }

    pub fn armor_ids(&self) -> [Option<String>; 4] {
        self.armor.clone().map(|stack| stack.map(|stack| stack.id))
    }

    pub fn armor_defense(&self) -> (f32, f32) {
        self.armor
            .iter()
            .flatten()
            .map(|stack| armor_value(&stack.id))
            .fold((0.0, 0.0), |(a, t), (da, dt)| (a + da, t + dt))
    }

    pub fn holds_melee_weapon(&self) -> bool {
        self.main_hand
            .as_ref()
            .is_some_and(|stack| melee_weapon(&stack.id))
    }

    pub fn holds_crossbow(&self) -> bool {
        self.main_hand
            .as_ref()
            .is_some_and(|stack| stack.id == "minecraft:crossbow")
    }

    pub fn crossbow_charged(&self) -> bool {
        self.main_hand.as_ref().is_some_and(|stack| {
            stack.id == "minecraft:crossbow"
                && stack
                    .components
                    .as_ref()
                    .and_then(|c| c.get("minecraft:charged_projectiles"))
                    .and_then(|p| p.as_array())
                    .is_some_and(|p| !p.is_empty())
        })
    }

    pub fn arm_pose(&self, offhand_loved: bool) -> ArmPose {
        if self.dancing {
            ArmPose::Dancing
        } else if offhand_loved {
            ArmPose::AdmiringItem
        } else if self.aggressive && self.holds_melee_weapon() {
            ArmPose::AttackingWithMeleeWeapon
        } else if self.charging_crossbow {
            ArmPose::CrossbowCharge
        } else if self.crossbow_charged() {
            ArmPose::CrossbowHold
        } else {
            ArmPose::Default
        }
    }

    pub fn hurt_generic(&mut self, amount: f32) -> DamageResult {
        let max = self.max_health();
        self.damage.hurt_generic(&mut self.health, max, amount)
    }
}

pub fn max_health(kind: PiglinKind) -> f32 {
    match kind {
        PiglinKind::Piglin => MAX_HEALTH,
        PiglinKind::Brute => BRUTE_MAX_HEALTH,
    }
}

pub fn melee_weapon(id: &str) -> bool {
    let name = id.trim_start_matches("minecraft:");
    ["_sword", "_axe", "_pickaxe", "_shovel", "_hoe"]
        .iter()
        .any(|suffix| name.ends_with(suffix))
        || matches!(name, "mace" | "trident" | "shears")
}

pub fn armor_value(id: &str) -> (f32, f32) {
    let name = id.trim_start_matches("minecraft:");
    let Some((material, piece)) = name.rsplit_once('_') else {
        return (0.0, 0.0);
    };
    let index = match piece {
        "boots" => 0,
        "leggings" => 1,
        "chestplate" => 2,
        "helmet" => 3,
        _ => return (0.0, 0.0),
    };
    let (defense, toughness): ([f32; 4], f32) = match material {
        "leather" => ([1.0, 2.0, 3.0, 1.0], 0.0),
        "copper" => ([1.0, 3.0, 4.0, 2.0], 0.0),
        "golden" => ([1.0, 3.0, 5.0, 2.0], 0.0),
        "chainmail" => ([1.0, 4.0, 5.0, 2.0], 0.0),
        "iron" => ([2.0, 5.0, 6.0, 2.0], 0.0),
        "diamond" => ([3.0, 6.0, 8.0, 3.0], 2.0),
        "netherite" => ([3.0, 6.0, 8.0, 3.0], 3.0),
        "turtle" if piece == "helmet" => ([0.0, 0.0, 0.0, 2.0], 0.0),
        _ => return (0.0, 0.0),
    };
    (defense[index], toughness)
}

pub fn max_damage(id: &str) -> Option<u32> {
    Some(match id.trim_start_matches("minecraft:") {
        "crossbow" => 465,
        "golden_sword" | "golden_axe" | "golden_spear" | "golden_pickaxe" | "golden_shovel"
        | "golden_hoe" => 32,
        "golden_helmet" => 77,
        "golden_chestplate" => 112,
        "golden_leggings" => 105,
        "golden_boots" => 91,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_piglin_baby_is_049_by_098_and_faster() {
        let mut piglin = Piglin::new(PiglinKind::Piglin, DVec3::ZERO);
        assert_eq!((piglin.body.width, piglin.body.height), (0.6, 1.95));
        assert_eq!(piglin.eye_height(), 1.79);
        assert_eq!(piglin.health, 16.0);
        piglin.set_baby(true);
        assert_eq!((piglin.body.width, piglin.body.height), (0.49, 0.98));
        assert_eq!(piglin.eye_height(), 0.78);
        assert!((piglin.movement_speed() - 0.42).abs() < 1.0e-6);
        assert_eq!(piglin.attack_damage(), 5.0);
        assert_eq!(piglin.follow_range(), 16.0);
    }

    #[test]
    fn brutes_have_fifty_health_and_twelve_follow_range() {
        let mut brute = Piglin::new(PiglinKind::Brute, DVec3::ZERO);
        assert_eq!(brute.health, 50.0);
        assert_eq!(brute.follow_range(), 12.0);
        assert_eq!(brute.attack_damage(), 7.0);
        brute.set_baby(true);
        assert!(!brute.is_baby());
        assert_eq!((brute.body.width, brute.body.height), (0.6, 1.95));
        assert_eq!(
            PiglinKind::from_type("minecraft:piglin_brute"),
            Some(PiglinKind::Brute)
        );
    }

    #[test]
    fn the_arm_pose_goes_dancing_admiring_melee_charging_then_holding() {
        let mut piglin = Piglin::new(PiglinKind::Piglin, DVec3::ZERO);
        assert_eq!(piglin.arm_pose(false), ArmPose::Default);
        piglin.main_hand = Some(ItemStack::new("minecraft:golden_sword", 1));
        assert_eq!(piglin.arm_pose(false), ArmPose::Default);
        piglin.aggressive = true;
        assert_eq!(piglin.arm_pose(false), ArmPose::AttackingWithMeleeWeapon);
        assert_eq!(piglin.arm_pose(true), ArmPose::AdmiringItem);
        piglin.dancing = true;
        assert_eq!(piglin.arm_pose(true), ArmPose::Dancing);
        let mut crossbow = Piglin::new(PiglinKind::Piglin, DVec3::ZERO);
        let mut stack = ItemStack::new("minecraft:crossbow", 1);
        crossbow.main_hand = Some(stack.clone());
        crossbow.aggressive = true;
        assert_eq!(crossbow.arm_pose(false), ArmPose::Default);
        crossbow.charging_crossbow = true;
        assert_eq!(crossbow.arm_pose(false), ArmPose::CrossbowCharge);
        crossbow.charging_crossbow = false;
        stack.components =
            Some(serde_json::json!({"minecraft:charged_projectiles": [{"id": "minecraft:arrow"}]}));
        crossbow.main_hand = Some(stack);
        assert_eq!(crossbow.arm_pose(false), ArmPose::CrossbowHold);
    }

    #[test]
    fn golden_armour_adds_its_defense() {
        let mut piglin = Piglin::new(PiglinKind::Piglin, DVec3::ZERO);
        piglin.armor[0] = Some(ItemStack::new("minecraft:golden_helmet", 1));
        piglin.armor[3] = Some(ItemStack::new("minecraft:golden_boots", 1));
        assert_eq!(piglin.armor_defense(), (3.0, 0.0));
        assert_eq!(
            piglin.equipment(HEAD).unwrap().id,
            "minecraft:golden_helmet"
        );
        assert_eq!(piglin.equipment(FEET).unwrap().id, "minecraft:golden_boots");
        assert!(piglin.equipment(CHEST).is_none());
    }
}
