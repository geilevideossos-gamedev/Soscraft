use super::Level;
use super::physics::{Aabb, collide_shapes};
use minecraftoss_core::BlockPos;
use minecraftoss_core::entity_data::wrap_degrees;
use minecraftoss_core::nbt::Tag;
use minecraftoss_core::random::RandomSource;
use std::collections::BTreeMap;

pub const BUCKET_ENTITY_DATA: &str = "minecraft:bucket_entity_data";
pub const CUSTOM_NAME: &str = "minecraft:custom_name";
pub const FROM_BUCKET: &str = "FromBucket";
pub const HEALTH: &str = "Health";
pub const NO_AI: &str = "NoAI";
pub const PERSISTENCE_REQUIRED: &str = "PersistenceRequired";
pub const FLAGS: [&str; 6] = [
    NO_AI,
    "Silent",
    "NoGravity",
    "Glowing",
    "Invulnerable",
    PERSISTENCE_REQUIRED,
];
pub const EMPTY_FISH: &str = "minecraft:item.bucket.empty_fish";
pub const FILL_FISH: &str = "minecraft:item.bucket.fill_fish";
const ENTITY_PLACE: &str = "minecraft:entity_place";
const FALLBACK_WIDTH: f32 = 0.6;

pub type Converter = fn(&Tag, &mut Vec<(String, Tag)>);

pub struct MobBucket {
    pub item: &'static str,
    pub kind: &'static str,
    pub water: bool,
    pub empty_sound: &'static str,
    pub fill_sound: &'static str,
    pub keys: &'static [&'static str],
    pub components: Converter,
}

pub fn no_components(_components: &Tag, _data: &mut Vec<(String, Tag)>) {}

pub const MOB_BUCKETS: &[MobBucket] = &[
    MobBucket {
        item: "minecraft:cod_bucket",
        kind: "minecraft:cod",
        water: true,
        empty_sound: EMPTY_FISH,
        fill_sound: FILL_FISH,
        keys: &[],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:salmon_bucket",
        kind: "minecraft:salmon",
        water: true,
        empty_sound: EMPTY_FISH,
        fill_sound: FILL_FISH,
        keys: &[],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:tropical_fish_bucket",
        kind: "minecraft:tropical_fish",
        water: true,
        empty_sound: EMPTY_FISH,
        fill_sound: FILL_FISH,
        keys: &[],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:pufferfish_bucket",
        kind: "minecraft:pufferfish",
        water: true,
        empty_sound: EMPTY_FISH,
        fill_sound: FILL_FISH,
        keys: &[],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:axolotl_bucket",
        kind: "minecraft:axolotl",
        water: true,
        empty_sound: "minecraft:item.bucket.empty_axolotl",
        fill_sound: "minecraft:item.bucket.fill_axolotl",
        keys: &["Age", "AgeLocked", "HuntingCooldown"],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:tadpole_bucket",
        kind: "minecraft:tadpole",
        water: true,
        empty_sound: "minecraft:item.bucket.empty_tadpole",
        fill_sound: "minecraft:item.bucket.fill_tadpole",
        keys: &["Age", "AgeLocked"],
        components: no_components,
    },
    MobBucket {
        item: "minecraft:sulfur_cube_bucket",
        kind: "minecraft:sulfur_cube",
        water: false,
        empty_sound: "minecraft:item.bucket.empty_sulfur_cube",
        fill_sound: "minecraft:item.bucket.fill_sulfur_cube",
        keys: &[],
        components: no_components,
    },
];

pub fn mob_bucket(item: &str) -> Option<&'static MobBucket> {
    MOB_BUCKETS.iter().find(|bucket| bucket.item == item)
}

pub fn bucket_of(kind: &str) -> Option<&'static MobBucket> {
    MOB_BUCKETS.iter().find(|bucket| bucket.kind == kind)
}

pub fn bucket_entity_data(
    bucket: &MobBucket,
    no_ai: bool,
    persistent: bool,
    health: f32,
    fields: &[(&str, Tag)],
) -> Tag {
    let mut data = BTreeMap::new();
    if no_ai {
        data.insert(NO_AI.to_owned(), Tag::Byte(1));
    }
    if persistent {
        data.insert(PERSISTENCE_REQUIRED.to_owned(), Tag::Byte(1));
    }
    data.insert(HEALTH.to_owned(), Tag::Float(health));
    for (key, value) in fields {
        if bucket.keys.iter().any(|kept| kept == key) {
            data.insert((*key).to_owned(), value.clone());
        }
    }
    Tag::Compound(data)
}

pub fn entity_data(bucket: &MobBucket, components: Option<&Tag>) -> Vec<(String, Tag)> {
    let saved = components.and_then(|components| components.get(BUCKET_ENTITY_DATA));
    let value = |key: &str| saved.and_then(|saved| saved.get(key));
    let mut data = Vec::new();
    for key in FLAGS {
        let Some(on) = value(key).and_then(truth) else {
            continue;
        };
        if on || key != PERSISTENCE_REQUIRED {
            data.push((key.to_owned(), Tag::Byte(i8::from(on))));
        }
    }
    if let Some(health) = value(HEALTH).and_then(Tag::as_f64) {
        data.push((HEALTH.to_owned(), Tag::Float(health as f32)));
    }
    for key in bucket.keys {
        if let Some(kept) = value(key) {
            data.push(((*key).to_owned(), kept.clone()));
        }
    }
    if let Some(components) = components {
        if let Some(name) = components.get(CUSTOM_NAME) {
            data.push(("CustomName".to_owned(), name.clone()));
        }
        (bucket.components)(components, &mut data);
    }
    data.push((FROM_BUCKET.to_owned(), Tag::Byte(1)));
    data
}

fn truth(tag: &Tag) -> Option<bool> {
    tag.as_f64().map(|value| value as i64 as i8 != 0)
}

impl Level<'_> {
    pub fn empty_mob_bucket(
        &mut self,
        pos: BlockPos,
        at: BlockPos,
        item: &str,
        components: Option<&Tag>,
    ) -> bool {
        let Some(bucket) = mob_bucket(item) else {
            return false;
        };
        if bucket.water && !self.empty_bucket_into(pos, true) {
            return false;
        }
        self.let_out(bucket, at, components);
        true
    }

    fn let_out(&mut self, bucket: &MobBucket, at: BlockPos, components: Option<&Tag>) {
        let width = crate::natural_spawner::type_width(bucket.kind).unwrap_or(FALLBACK_WIDTH);
        let position = [
            f64::from(at.x) + 0.5,
            f64::from(at.y) + self.settle(at, width),
            f64::from(at.z) + 0.5,
        ];
        let y_rot = wrap_degrees(self.random.next_f32() * 360.0);
        let Ok(mut tag) = self.summon_mob(bucket.kind, position, None, y_rot) else {
            return;
        };
        if let Tag::Compound(map) = &mut tag {
            map.extend(entity_data(bucket, components));
        }
        self.spawned.push(tag);
        self.block_game_event(ENTITY_PLACE, at, None);
    }

    fn settle(&self, at: BlockPos, width: f32) -> f64 {
        let (x, y, z) = (f64::from(at.x), f64::from(at.y), f64::from(at.z));
        let shapes = self.block_collisions(&Aabb::new(x, y, z, x + 1.0, y + 1.0, z + 1.0));
        let moving = Aabb::for_entity(x + 0.5, y + 1.0, z + 0.5, width, width);
        1.0 + collide_shapes(1, &moving, &shapes, -1.0)
    }
}

#[cfg(test)]
mod tests;
