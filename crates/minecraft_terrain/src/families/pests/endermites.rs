use super::endermite_render;
use crate::client_mobs::ClientMobs;
use crate::families::{ClientFx, FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::endermites::Endermite;

impl TerrainFamily for Endermite {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["Lifetime"];
    const TEXTURES: &'static [&'static str] = &[endermite_render::TEXTURE];
    const SHADOW_RADIUS: f32 = endermite_render::SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        mob.state.ai.state.look_control.head_yaw = mob.core.yaw;
        mob.state.life = tag
            .get("Lifetime")
            .and_then(Tag::as_i64)
            .map_or(0, |life| life as i32);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        vec![("Lifetime", Tag::Int(mob.state.life))]
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
        let ids = shown.map(|mob| mob.core.id);
        endermite_render::append_endermites(out.models, ids, poses, atlas, light, partial);
    }

    fn client_effects(mobs: &[FamilyMob<Self>], poses: &ClientMobs, fx: &mut ClientFx<'_>) {
        for mob in mobs.iter().filter(|mob| !mob.core.removed) {
            let Some(shown) = poses.get(mob.core.id) else {
                continue;
            };
            for (start, motion) in endermite_render::portal_sparks(shown.position, fx.random) {
                fx.portal.emit(start, motion);
            }
        }
    }
}
