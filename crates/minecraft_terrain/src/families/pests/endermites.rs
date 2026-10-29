use crate::families::TerrainFamily;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
pub use minecraftoss_entities::world::families::pests::endermites::Endermite;

impl TerrainFamily for Endermite {
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
}
