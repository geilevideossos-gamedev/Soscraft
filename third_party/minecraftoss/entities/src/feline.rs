use crate::age::Age;
use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;

pub const MAX_HEALTH: f32 = 10.0;
pub const MOVEMENT_SPEED: f64 = 0.300_000_011_920_928_96;
pub const ATTACK_DAMAGE: f32 = 3.0;
pub const FOLLOW_RANGE: f64 = 16.0;
pub const WIDTH: f32 = 0.6;
pub const HEIGHT: f32 = 0.7;
pub const CAT_EYE_HEIGHT: f32 = 0.35;
pub const OCELOT_EYE_HEIGHT: f32 = 0.595;
pub const BABY_WIDTH: f32 = 0.3;
pub const BABY_HEIGHT: f32 = 0.35;
pub const BABY_EYE_HEIGHT: f32 = 0.343_75;
pub const CAT_AMBIENT_INTERVAL: i32 = 120;
pub const OCELOT_AMBIENT_INTERVAL: i32 = 900;
pub const TEMPT_SPEED: f64 = 0.6;
pub const WALK_SPEED: f64 = 0.8;
pub const SPRINT_SPEED: f64 = 1.33;
pub const SPRINT_SPEED_BONUS: f64 = 0.3;
pub const DEFAULT_COLLAR: u8 = 14;
pub const DEFAULT_CAT_VARIANT: &str = "black";
pub const CAT_VARIANTS: [&str; 11] = [
    "all_black",
    "black",
    "british_shorthair",
    "calico",
    "jellie",
    "persian",
    "ragdoll",
    "red",
    "siamese",
    "tabby",
    "white",
];
pub const CAT_SOUND_VARIANTS: [&str; 2] = ["classic", "royal"];
pub const FOOD_TAG: &str = "minecraft:cat_food";
pub const OCELOT_FOOD_TAG: &str = "minecraft:ocelot_food";
pub const TRUSTING_EVENT: u8 = 41;
pub const DISTRUSTING_EVENT: u8 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FelineKind {
    Cat,
    Ocelot,
}

impl FelineKind {
    pub fn type_id(self) -> &'static str {
        match self {
            Self::Cat => "minecraft:cat",
            Self::Ocelot => "minecraft:ocelot",
        }
    }

    pub fn of_type(kind: &str) -> Option<Self> {
        match kind {
            "minecraft:cat" => Some(Self::Cat),
            "minecraft:ocelot" => Some(Self::Ocelot),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FelineSounds {
    pub ambient: &'static str,
    pub beg_for_food: &'static str,
    pub death: &'static str,
    pub eat: &'static str,
    pub hiss: &'static str,
    pub hurt: &'static str,
    pub purr: &'static str,
    pub purreow: &'static str,
    pub stray_ambient: &'static str,
}

macro_rules! cat_sounds {
    ($prefix:literal) => {
        FelineSounds {
            ambient: concat!($prefix, ".ambient"),
            beg_for_food: concat!($prefix, ".beg_for_food"),
            death: concat!($prefix, ".death"),
            eat: concat!($prefix, ".eat"),
            hiss: concat!($prefix, ".hiss"),
            hurt: concat!($prefix, ".hurt"),
            purr: concat!($prefix, ".purr"),
            purreow: concat!($prefix, ".purreow"),
            stray_ambient: concat!($prefix, ".stray_ambient"),
        }
    };
}

const OCELOT_SOUNDS: FelineSounds = FelineSounds {
    ambient: "entity.ocelot.ambient",
    beg_for_food: "entity.ocelot.ambient",
    death: "entity.ocelot.death",
    eat: "entity.generic.eat",
    hiss: "entity.ocelot.ambient",
    hurt: "entity.ocelot.hurt",
    purr: "entity.ocelot.ambient",
    purreow: "entity.ocelot.ambient",
    stray_ambient: "entity.ocelot.ambient",
};

pub fn cat_sounds(variant: &str, baby: bool) -> FelineSounds {
    if baby {
        return cat_sounds!("entity.baby_cat");
    }
    match variant {
        "royal" => cat_sounds!("entity.cat_royal"),
        _ => cat_sounds!("entity.cat"),
    }
}

pub fn is_food(item: &str) -> bool {
    matches!(item, "minecraft:cod" | "minecraft:salmon")
}

#[derive(Clone, Debug)]
pub struct Feline {
    pub kind: FelineKind,
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub age: Age,
    pub variant: String,
    pub sound_variant: String,
    pub collar: u8,
    pub tame: bool,
    pub owner: Option<u128>,
    pub ordered_to_sit: bool,
    pub sitting: bool,
    pub trusting: bool,
    pub in_love: i32,
    pub crouching: bool,
    pub sprinting: bool,
    pub lying: bool,
    pub relax_state_one: bool,
    pub lying_on_sleeping_player: bool,
    pub lie_down_amount: f32,
    pub lie_down_amount_o: f32,
    pub lie_down_amount_tail: f32,
    pub lie_down_amount_o_tail: f32,
    pub relax_state_one_amount: f32,
    pub relax_state_one_amount_o: f32,
}

impl Feline {
    pub fn new(kind: FelineKind, position: DVec3) -> Self {
        Self {
            kind,
            body: Body::new(position, WIDTH, HEIGHT),
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            age: Age::default(),
            variant: DEFAULT_CAT_VARIANT.to_owned(),
            sound_variant: CAT_SOUND_VARIANTS[0].to_owned(),
            collar: DEFAULT_COLLAR,
            tame: false,
            owner: None,
            ordered_to_sit: false,
            sitting: false,
            trusting: false,
            in_love: 0,
            crouching: false,
            sprinting: false,
            lying: false,
            relax_state_one: false,
            lying_on_sleeping_player: false,
            lie_down_amount: 0.0,
            lie_down_amount_o: 0.0,
            lie_down_amount_tail: 0.0,
            lie_down_amount_o_tail: 0.0,
            relax_state_one_amount: 0.0,
            relax_state_one_amount_o: 0.0,
        }
    }

    pub fn is_cat(&self) -> bool {
        self.kind == FelineKind::Cat
    }

    pub fn baby(&self) -> bool {
        self.age.baby()
    }

    pub fn set_age(&mut self, ticks: i32) {
        self.age.set(ticks);
        let (width, height) = if self.age.baby() {
            (BABY_WIDTH, BABY_HEIGHT)
        } else {
            (WIDTH, HEIGHT)
        };
        self.body.width = width;
        self.body.height = height;
    }

    pub fn eye_height(&self) -> f32 {
        match (self.baby(), self.kind) {
            (true, _) => BABY_EYE_HEIGHT,
            (false, FelineKind::Cat) => CAT_EYE_HEIGHT,
            (false, FelineKind::Ocelot) => OCELOT_EYE_HEIGHT,
        }
    }

    pub fn max_health(&self) -> f32 {
        MAX_HEALTH
    }

    pub fn ambient_interval(&self) -> i32 {
        match self.kind {
            FelineKind::Cat => CAT_AMBIENT_INTERVAL,
            FelineKind::Ocelot => OCELOT_AMBIENT_INTERVAL,
        }
    }

    pub fn sounds(&self) -> FelineSounds {
        match self.kind {
            FelineKind::Cat => cat_sounds(&self.sound_variant, self.baby()),
            FelineKind::Ocelot => OCELOT_SOUNDS,
        }
    }

    pub fn texture(&self) -> String {
        let baby = if self.baby() { "_baby" } else { "" };
        match self.kind {
            FelineKind::Cat => format!(
                "minecraft:entity/cat/cat_{}{baby}",
                self.variant.trim_start_matches("minecraft:")
            ),
            FelineKind::Ocelot => format!("minecraft:entity/cat/ocelot{baby}"),
        }
    }

    pub fn shows_collar(&self) -> bool {
        self.is_cat() && self.tame
    }

    pub fn wild(&self) -> bool {
        match self.kind {
            FelineKind::Cat => !self.tame,
            FelineKind::Ocelot => !self.trusting,
        }
    }

    pub fn update_lie_down_amount(&mut self) {
        self.lie_down_amount_o = self.lie_down_amount;
        self.lie_down_amount_o_tail = self.lie_down_amount_tail;
        if self.lying {
            self.lie_down_amount = (self.lie_down_amount + 0.15).min(1.0);
            self.lie_down_amount_tail = (self.lie_down_amount_tail + 0.08).min(1.0);
        } else {
            self.lie_down_amount = (self.lie_down_amount - 0.22).max(0.0);
            self.lie_down_amount_tail = (self.lie_down_amount_tail - 0.13).max(0.0);
        }
    }

    pub fn update_relax_state_one_amount(&mut self) {
        self.relax_state_one_amount_o = self.relax_state_one_amount;
        if self.relax_state_one {
            self.relax_state_one_amount = (self.relax_state_one_amount + 0.1).min(1.0);
        } else {
            self.relax_state_one_amount = (self.relax_state_one_amount - 0.13).max(0.0);
        }
    }

    pub fn lie_down(&self, partial: f32) -> (f32, f32, f32) {
        let lerp = |o: f32, n: f32| o + (n - o) * partial;
        (
            lerp(self.lie_down_amount_o, self.lie_down_amount),
            lerp(self.lie_down_amount_o_tail, self.lie_down_amount_tail),
            lerp(self.relax_state_one_amount_o, self.relax_state_one_amount),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cat_and_an_ocelot_take_their_sizes_textures_and_voices() {
        let mut cat = Feline::new(FelineKind::Cat, DVec3::ZERO);
        assert_eq!(
            (
                cat.body.width,
                cat.body.height,
                cat.eye_height(),
                cat.health
            ),
            (0.6, 0.7, 0.35, 10.0)
        );
        assert_eq!(cat.texture(), "minecraft:entity/cat/cat_black");
        cat.variant = "minecraft:british_shorthair".to_owned();
        cat.set_age(Age::BABY_START);
        assert_eq!(
            (cat.body.width, cat.body.height, cat.eye_height()),
            (0.3, 0.35, 0.343_75)
        );
        assert_eq!(
            cat.texture(),
            "minecraft:entity/cat/cat_british_shorthair_baby"
        );
        assert_eq!(cat.sounds().purr, "entity.baby_cat.purr");
        cat.set_age(0);
        cat.sound_variant = "royal".to_owned();
        assert_eq!(cat.sounds().hiss, "entity.cat_royal.hiss");
        let ocelot = Feline::new(FelineKind::Ocelot, DVec3::ZERO);
        assert_eq!(
            (ocelot.eye_height(), ocelot.ambient_interval()),
            (0.595, 900)
        );
        assert_eq!(ocelot.texture(), "minecraft:entity/cat/ocelot");
        assert_eq!(ocelot.sounds().hurt, "entity.ocelot.hurt");
        assert!(
            is_food("minecraft:cod")
                && is_food("minecraft:salmon")
                && !is_food("minecraft:tropical_fish")
        );
        assert!(!ocelot.shows_collar() && ocelot.wild());
    }

    #[test]
    fn lying_down_eases_in_and_out_at_the_vanilla_rates() {
        let mut cat = Feline::new(FelineKind::Cat, DVec3::ZERO);
        cat.lying = true;
        cat.update_lie_down_amount();
        assert!(
            (cat.lie_down_amount - 0.15).abs() < 1e-6
                && (cat.lie_down_amount_tail - 0.08).abs() < 1e-6
        );
        for _ in 0..20 {
            cat.update_lie_down_amount();
        }
        assert_eq!((cat.lie_down_amount, cat.lie_down_amount_tail), (1.0, 1.0));
        cat.lying = false;
        cat.update_lie_down_amount();
        assert!(
            (cat.lie_down_amount - 0.78).abs() < 1e-6
                && (cat.lie_down_amount_tail - 0.87).abs() < 1e-6
        );
        cat.relax_state_one = true;
        cat.update_relax_state_one_amount();
        assert!((cat.relax_state_one_amount - 0.1).abs() < 1e-6);
        let (lie, _, relax) = cat.lie_down(0.5);
        assert!((lie - 0.89).abs() < 1e-6 && (relax - 0.05).abs() < 1e-6);
    }
}
