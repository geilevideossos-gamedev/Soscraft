use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::night::creakings::Creaking;

pub const SHADOW_RADIUS: f32 = 0.6;

impl TerrainFamily for Creaking {
    const TRACKING_RANGE: i32 = 8;
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
