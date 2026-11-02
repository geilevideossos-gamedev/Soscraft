use crate::bottle::filled_result;
use crate::bucket::{
    BUCKET, Change, EMPTY_SOUND, Fluid, ItemUse, Materials, Sound, WATER_BUCKET, aim, bucket_stack,
    liquid_container, offset, pour,
};
use crate::families::TerrainFamily;
use crate::item_nbt::tag_json;
use crate::mob_actions::{Actor, MobOutcome, MobSound};
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::world::families::FamilyMob;
use minecraftoss_entities::world::{EntityWorld, LEAD};
use minecraftoss_player::inventory::ItemStack;
use minecraftoss_player::{Player, Pos, World};
use minecraftoss_world::level::mob_bucket::{
    BUCKET_ENTITY_DATA, MobBucket, bucket_entity_data, bucket_of, mob_bucket,
};
use serde_json::Value;

#[derive(Clone, Debug, PartialEq)]
pub struct MobBucketUse {
    pub pos: Pos,
    pub at: Pos,
    pub item: &'static str,
    pub components: Option<Tag>,
}

pub trait Bucketable: TerrainFamily {
    const PICKUP_ITEM: &'static str = WATER_BUCKET;

    fn bucket(kind: &str) -> Option<&'static MobBucket> {
        bucket_of(kind)
    }

    fn bucket_components(_mob: &FamilyMob<Self>) -> Vec<(&'static str, Value)> {
        Vec::new()
    }
}

pub fn pickup<S: Bucketable>(
    world: &mut EntityWorld,
    id: u64,
    actor: &mut Actor,
    outcome: &mut MobOutcome,
) -> bool {
    let held = actor
        .inventory
        .slots
        .get(actor.selected)
        .and_then(Option::as_ref);
    if held.is_none_or(|stack| stack.id != S::PICKUP_ITEM) {
        return false;
    }
    let Some(mob) = world.family::<S>().iter().find(|mob| mob.core.id == id) else {
        return false;
    };
    let core = &mob.core;
    let alive = !core.removed && core.health > 0.0;
    let Some(bucket) = S::bucket(core.kind).filter(|_| alive) else {
        return false;
    };
    let data = bucket_entity_data(
        bucket,
        core.no_ai,
        core.persistent,
        core.health,
        &S::fields(mob),
    );
    let mut components = serde_json::Map::new();
    components.insert(BUCKET_ENTITY_DATA.to_owned(), tag_json(&data));
    for (key, value) in S::bucket_components(mob) {
        components.insert(key.to_owned(), value);
    }
    let mut filled = bucket_stack(bucket.item);
    filled.components = Some(Value::Object(components));
    let position = core.body.position;
    outcome.sounds.push(MobSound {
        event: bucket.fill_sound.to_owned(),
        position,
        volume: 1.0,
        pitch: 1.0,
        category: S::voice(mob).2,
    });
    let rest = if actor.infinite {
        actor.inventory.add_item(filled, actor.selected)
    } else {
        filled_result(actor.inventory, actor.selected, filled)
    };
    outcome.drops.extend(rest.map(|rest| (rest, position)));
    world.drop_leash(id, true);
    let leads = world.take_lead_drops().into_iter();
    outcome
        .drops
        .extend(leads.map(|at| (ItemStack::new(LEAD, 1), at)));
    if let Some(mob) = world.family_mob_mut::<S>(id) {
        mob.core.removed = true;
    }
    outcome.inventory_changed = true;
    true
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
