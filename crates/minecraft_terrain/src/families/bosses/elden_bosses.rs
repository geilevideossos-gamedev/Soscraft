use crate::client_mobs::ClientMobs;
use crate::families::{FamilyDraw, TerrainFamily};
use crate::lighting::SkyLight;
use crate::mesh::Atlas;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::bosses::elden_bosses::EldenBoss;

pub const TRACKING_RANGE: i32 = 10;
pub const SHADOW_RADIUS: f32 = 1.0;

impl TerrainFamily for EldenBoss {
    const TRACKING_RANGE: i32 = TRACKING_RANGE;
    const TEXTURES: &'static [&'static str] = &[];
    const SHADOW_RADIUS: f32 = SHADOW_RADIUS;

    fn read(mob: &mut FamilyMob<Self>, tag: &Tag) {
        crate::elden_save::read(mob, tag);
    }

    fn fields(mob: &FamilyMob<Self>) -> Vec<(&'static str, Tag)> {
        crate::elden_save::fields(mob)
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
