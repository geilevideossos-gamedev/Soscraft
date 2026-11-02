use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, FamilyShow, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use crate::mob_actions::{Actor, MobOutcome, MobSound};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::age::Age;
use minecraftoss_entities::animal::{self, AnimalEvent};
use minecraftoss_entities::cow::InteractionResult;
use minecraftoss_entities::world::EntityWorld;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::water::squids::Squid;
use minecraftoss_entities::world::families::water::squids::{self, GLOW_SQUID};

mod render;

pub const GLOW_TRACKING_RANGE: i32 = 10;
const TENTACLE_ANGLE: usize = 0;
const X_BODY_ROT: usize = 1;
const Z_BODY_ROT: usize = 2;
const Y_BODY_ROT: usize = 3;
const INK_SPURTS: usize = 4;
const DARK_TICKS: usize = 5;

fn int(tag: &Tag, key: &str) -> Option<i32> {
    tag.get(key).and_then(Tag::as_i64).map(|value| value as i32)
}

fn bits(value: f32) -> i32 {
    value.to_bits() as i32
}

fn float(value: i32) -> f32 {
    f32::from_bits(value as u32)
}

impl TerrainFamily for Squid {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["AgeLocked", "DarkTicksRemaining"];
    const TEXTURES: &'static [&'static str] = &render::TEXTURES;
    const SHADOW_RADIUS: f32 = render::SHADOW_RADIUS;
    const CAGE_MARGIN: f32 = render::CAGE_MARGIN;

    fn tracking_range(kind: &'static str) -> i32 {
        if kind == GLOW_SQUID {
            GLOW_TRACKING_RANGE
        } else {
            Self::TRACKING_RANGE
        }
    }

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        let shown_baby = int(tag, "Baby").is_some_and(|baby| baby != 0);
        let ticks = int(tag, "Age").unwrap_or(if shown_baby { Age::BABY_START } else { 0 });
        mob.state.age = Age {
            ticks,
            forced: int(tag, "ForcedAge").unwrap_or(0),
            locked: int(tag, "AgeLocked").is_some_and(|locked| locked != 0),
            ..Age::default()
        };
        if mob.state.glow {
            mob.state.dark_ticks = int(tag, "DarkTicksRemaining").unwrap_or(0);
        }
        mob.state.y_body_rot_o = mob.core.yaw;
        let baby = mob.state.age.baby();
        squids::set_baby(mob, baby);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        let age = &mob.state.age;
        let mut fields = vec![
            ("Age", Tag::Int(age.ticks)),
            ("ForcedAge", Tag::Int(age.forced)),
            ("AgeLocked", Tag::Byte(i8::from(age.locked))),
        ];
        if mob.state.glow {
            fields.push(("DarkTicksRemaining", Tag::Int(mob.state.dark_ticks)));
        }
        fields
    }

    fn show(mob: &FamilyMob<Self>) -> FamilyShow {
        let state = &mob.state;
        let mut show = FamilyShow::default();
        show.values[TENTACLE_ANGLE] = bits(state.tentacle_angle);
        show.values[X_BODY_ROT] = bits(state.x_body_rot);
        show.values[Z_BODY_ROT] = bits(state.z_body_rot);
        show.values[Y_BODY_ROT] = bits(mob.core.yaw);
        show.values[INK_SPURTS] = state.ink_spurts as i32;
        show.values[DARK_TICKS] = state.dark_ticks;
        show
    }

    fn show_on(mob: &mut FamilyMob<Self>, show: &FamilyShow) {
        let FamilyMob { core, state } = mob;
        state.tentacle_angle_old = state.tentacle_angle;
        state.x_body_rot_o = state.x_body_rot;
        state.z_body_rot_o = state.z_body_rot;
        state.y_body_rot_o = core.yaw;
        state.tentacle_angle = float(show.values[TENTACLE_ANGLE]);
        state.x_body_rot = float(show.values[X_BODY_ROT]);
        state.z_body_rot = float(show.values[Z_BODY_ROT]);
        core.yaw = float(show.values[Y_BODY_ROT]);
        state.ink_spurts = show.values[INK_SPURTS] as u32;
        state.dark_ticks = show.values[DARK_TICKS];
    }

    fn draw(
        out: &mut FamilyDraw<'_>,
        mobs: &[FamilyMob<Self>],
        poses: &ClientMobs,
        atlas: &Atlas,
        light: &SkyLight,
        partial: f32,
    ) {
        let shown = mobs.iter().filter(|mob| !mob.core.removed);
        render::append_squids(out.culled, shown, poses, atlas, light, partial);
    }

    fn interact(
        world: &mut EntityWorld,
        id: u64,
        actor: &mut Actor,
        outcome: &mut MobOutcome,
    ) -> bool {
        let Some(mob) = world.family_mob_mut::<Self>(id) else {
            return false;
        };
        if !mob.core.alive() {
            return false;
        }
        let position = mob.core.body.position;
        let FamilyMob { core, state } = mob;
        let mut in_love = 0;
        let (result, events) = animal::interact(
            &mut state.age,
            &mut in_love,
            &mut core.persistent,
            actor.inventory,
            actor.selected,
            actor.infinite,
            |_: &str| false,
            false,
        );
        if result == InteractionResult::Pass {
            return false;
        }
        outcome.inventory_changed = true;
        let lock = events.iter().find_map(|event| match event {
            AnimalEvent::AgeLock => Some("item.golden_dandelion.use"),
            AnimalEvent::AgeUnlock => Some("item.golden_dandelion.unuse"),
            AnimalEvent::Hearts => None,
        });
        if let Some(event) = lock {
            outcome.sounds.push(MobSound {
                event: event.to_owned(),
                position,
                volume: 1.0,
                pitch: 1.0,
                category: "players_volume",
            });
        }
        true
    }
}

#[cfg(test)]
mod tests;
