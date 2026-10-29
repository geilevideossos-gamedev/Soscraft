use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::deep_dark::wardens::Warden;

pub const TRACKING_RANGE: i32 = 16;
pub const SHADOW_RADIUS: f32 = 0.9;

impl TerrainFamily for Warden {
    const TRACKING_RANGE: i32 = TRACKING_RANGE;
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

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
