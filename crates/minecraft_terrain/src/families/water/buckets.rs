use crate::bucket::{
    BUCKET, Change, EMPTY_SOUND, Fluid, ItemUse, Materials, Sound, aim, bucket_stack,
    liquid_container, offset, pour,
};
use minecraftoss_core::nbt::Tag;
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Player, Pos, World};
use minecraftoss_world::level::mob_bucket::mob_bucket;

#[derive(Clone, Debug, PartialEq)]
pub struct MobBucketUse {
    pub pos: Pos,
    pub at: Pos,
    pub item: &'static str,
    pub components: Option<Tag>,
}

pub fn bucket_use(
    player: &Player,
    world: &impl World,
    materials: &impl Materials,
    held: &ItemStack,
    random: impl FnMut() -> f32,
) -> Option<ItemUse> {
    let bucket = mob_bucket(&held.id)?;
    let hit = aim(player, world, false)?;
    let into_clicked = bucket.water
        && world
            .block(hit.pos)
            .is_some_and(|block| liquid_container(&block));
    let at = if into_clicked {
        hit.pos
    } else {
        offset(hit.pos, hit.face.offset())
    };
    let (pos, sounds) = if bucket.water {
        let poured = pour(player, world, materials, Fluid::Water, random)?;
        let Change::Server(pos, _) = poured.change else {
            return None;
        };
        let splash = |sound: Sound| {
            if sound.event == EMPTY_SOUND {
                Sound {
                    event: bucket.empty_sound,
                    ..sound
                }
            } else {
                sound
            }
        };
        (pos, poured.sounds.into_iter().map(splash).collect())
    } else {
        (at, vec![Sound::at_block(bucket.empty_sound, at)])
    };
    let components = held
        .components
        .as_ref()
        .and_then(crate::item_nbt::components_tag);
    Some(ItemUse {
        change: Change::MobBucket(MobBucketUse {
            pos,
            at,
            item: bucket.item,
            components,
        }),
        result: Some(bucket_stack(BUCKET)),
        wear: 0,
        consumed: false,
        sounds,
    })
}

#[cfg(test)]
mod tests;
