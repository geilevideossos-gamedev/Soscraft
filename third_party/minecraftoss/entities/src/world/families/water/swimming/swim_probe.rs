use crate::tempt::PlayerCandidate;
use crate::world::EntityWorld;
use crate::world::emissions::play_movement;
use crate::world::families::{Breath, Family, FamilyMob, MobCore, movement_sounds};
use glam::DVec3;
use minecraftoss_player::World;

pub const KIND: &str = "minecraft:swim_probe";
pub const SILENT: &str = "minecraft:silent_swim_probe";
pub const SWIM: &str = "entity.fish.swim";
pub const SILENT_SPLASH: &str = "entity.hostile.splash";
pub const WIDTH: f32 = 0.5;
pub const HEIGHT: f32 = 0.3;
pub const EYE_HEIGHT: f32 = 0.195;
pub const MAX_HEALTH: f32 = 3.0;

#[derive(Clone)]
pub struct SwimProbe {
    pub breath: Breath,
    pub input: DVec3,
    pub speed: f32,
}

impl Family for SwimProbe {
    const KEY: u8 = 79;
    const KINDS: &'static [&'static str] = &[KIND, SILENT];

    fn spawn(kind: &'static str, position: DVec3) -> Option<FamilyMob<Self>> {
        let core = MobCore::new(kind, position, WIDTH, HEIGHT, EYE_HEIGHT, MAX_HEALTH);
        let state = SwimProbe {
            breath: Breath::Gills,
            input: DVec3::ZERO,
            speed: 0.0,
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
        if let Some(mob) = world
            .family_mob_mut::<Self>(id)
            .filter(|_| base.alive && !base.no_ai)
        {
            let sounds = movement_sounds::<Self>(mob.core.kind);
            let FamilyMob { core, state } = mob;
            let body = &mut core.body;
            body.trim_small_velocity();
            if body.touching_water {
                body.travel_water(&*level, state.input, core.yaw);
            } else {
                let _ = body.travel_air(&*level, state.input, state.speed, core.yaw);
            }
            play_movement(
                body,
                core.tick_count,
                &mut core.random,
                &mut core.voices,
                sounds,
            );
        }
        world.family_after_move::<Self, W>(id, level, base.game_time, ticks);
    }

    fn category(_kind: &str) -> &'static str {
        "water_ambient"
    }

    fn breath(mob: &FamilyMob<Self>) -> Breath {
        mob.state.breath
    }

    fn swim_sounds(kind: &str) -> Option<(&'static str, &'static str)> {
        if kind == SILENT {
            Some(("", SILENT_SPLASH))
        } else {
            Some((SWIM, "entity.generic.splash"))
        }
    }

    fn quiet_steps(kind: &str) -> bool {
        kind == KIND
    }
}
