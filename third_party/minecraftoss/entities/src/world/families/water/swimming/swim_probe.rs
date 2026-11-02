use crate::tempt::PlayerCandidate;
use crate::world::EntityWorld;
use crate::world::families::{Breath, Family, FamilyMob, MobCore};
use glam::DVec3;
use minecraftoss_player::World;

pub const KIND: &str = "minecraft:swim_probe";
pub const WIDTH: f32 = 0.5;
pub const HEIGHT: f32 = 0.3;
pub const EYE_HEIGHT: f32 = 0.195;
pub const MAX_HEALTH: f32 = 3.0;

#[derive(Clone)]
pub struct SwimProbe {
    pub breath: Breath,
}

impl Family for SwimProbe {
    const KEY: u8 = 79;
    const KINDS: &'static [&'static str] = &[KIND];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let state = SwimProbe {
            breath: Breath::Gills,
        };
        Some(FamilyMob { core, state })
    }

    fn tick<W: World>(
        world: &mut EntityWorld,
        id: u64,
        level: &mut W,
        players: &[PlayerCandidate],
        ticks: &dyn Fn(DVec3) -> bool,
    ) {
        let Some(base) = world.family_base_tick::<Self, W>(id, level, players) else {
            return;
        };
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn category(_kind: &str) -> &'static str {
        "water_ambient"
    }

    fn breath(mob: &FamilyMob<Self>) -> Breath {
        mob.state.breath
    }
}
