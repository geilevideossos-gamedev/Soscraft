use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::endermites::Endermite;

impl TerrainFamily for Endermite {
    const TRACKING_RANGE: i32 = 8;
    const VOLATILE: &'static [&'static str] = &["Lifetime"];
    const TEXTURES: &'static [&'static str] = &["minecraft:entity/endermite/endermite"];
    const SHADOW_RADIUS: f32 = 0.3;

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
        _out: &mut FamilyDraw<'_>,
        _mobs: &[FamilyMob<Self>],
        _poses: &ClientMobs,
        _atlas: &Atlas,
        _light: &SkyLight,
        _partial: f32,
    ) {
    }
}
