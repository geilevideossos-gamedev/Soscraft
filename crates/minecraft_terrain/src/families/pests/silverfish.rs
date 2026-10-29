use super::silverfish_render;
use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::silverfish::Silverfish;

impl TerrainFamily for Silverfish {
    const TRACKING_RANGE: i32 = 8;
    const TEXTURES: &'static [&'static str] = &[silverfish_render::TEXTURE];
    const SHADOW_RADIUS: f32 = silverfish_render::SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, _tag: &Tag) {
        mob.state.ai.state.look_control.head_yaw = mob.core.yaw;
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
        silverfish_render::append_silverfish(out.models, ids, poses, atlas, light, partial);
    }
}
