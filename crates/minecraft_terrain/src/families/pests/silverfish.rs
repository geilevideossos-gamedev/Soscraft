use crate::families::TerrainFamily;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::silverfish::Silverfish;

impl TerrainFamily for Silverfish {
    fn read(mob: &mut FamilyMob<Self>, _tag: &Tag) {
        mob.state.ai.state.look_control.head_yaw = mob.core.yaw;
    }
}
