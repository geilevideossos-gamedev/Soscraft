use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::silverfish::Silverfish;

impl TerrainFamily for Silverfish {
    const TRACKING_RANGE: i32 = 8;
    const TEXTURES: &'static [&'static str] = &["minecraft:entity/silverfish/silverfish"];
    const SHADOW_RADIUS: f32 = 0.3;

    fn read(mob: &mut FamilyMob<Self>, _tag: &Tag) {
        mob.state.ai.state.look_control.head_yaw = mob.core.yaw;
    }

    fn draw(
        _out: &mut FamilyDraw<'_>,
        _mobs: &[FamilyMob<Self>],
        _poses: &ClientMobs,
        _atlas: &Atlas,
        _light: &SkyLight,
        _partial: f32,
    ) {
    }
}
