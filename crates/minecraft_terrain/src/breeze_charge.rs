use crate::thrown::{PlayerBody, Thrown};
use glam::DVec3;
use minecraftoss_core::nbt::Tag;
use minecraftoss_entities::gossip::{uuid_from_ints, uuid_to_ints};
pub use minecraftoss_entities::world::families::trial::breezes::CHARGE as ENTITY;
use minecraftoss_entities::world::{PlayerHit, PlayerHitKind};
use minecraftoss_player::rng::LegacyRandom;
use std::collections::BTreeMap;

pub const HIT_MESSAGE: &str = "mob";

pub fn is_charge(thrown: &Thrown) -> bool {
    thrown.item == ENTITY
}

pub fn player_hit(
    player: u64,
    shooter: u64,
    velocity: DVec3,
    players: &[PlayerBody],
) -> Option<PlayerHit> {
    let feet = players.iter().find(|p| p.id == player)?.feet;
    Some(PlayerHit {
        player_id: player,
        damage: crate::wind::DAMAGE,
        kind: PlayerHitKind::Typed {
            damage_type: crate::wind::DAMAGE_TYPE,
            message: HIT_MESSAGE,
            from: feet - velocity.normalize_or_zero(),
            push: DVec3::ZERO,
            bypasses_armor: false,
            bypasses_shield: false,
            disable_shield_seconds: 0.0,
            effect: None,
        },
        source: Some(shooter),
    })
}

pub fn to_tag(charge: &Thrown, owner: Option<u128>) -> Option<Tag> {
    if !is_charge(charge) || !charge.alive() {
        return None;
    }
    let doubles = |v: DVec3| Tag::List(vec![Tag::Double(v.x), Tag::Double(v.y), Tag::Double(v.z)]);
    let mut map = BTreeMap::new();
    map.insert("id".to_owned(), Tag::String(ENTITY.to_owned()));
    map.insert("Pos".to_owned(), doubles(charge.position()));
    map.insert("Motion".to_owned(), doubles(charge.velocity()));
    map.insert(
        "Rotation".to_owned(),
        Tag::List(vec![Tag::Float(charge.yaw()), Tag::Float(charge.pitch())]),
    );
    if let Some(owner) = owner {
        map.insert(
            "Owner".to_owned(),
            Tag::IntArray(uuid_to_ints(owner).to_vec()),
        );
    }
    map.insert(
        "LeftOwner".to_owned(),
        Tag::Byte(i8::from(charge.left_owner())),
    );
    map.insert("HasBeenShot".to_owned(), Tag::Byte(1));
    map.insert("acceleration_power".to_owned(), Tag::Double(0.0));
    Some(Tag::Compound(map))
}

pub fn from_tag(tag: &Tag, random: &mut LegacyRandom) -> Option<Thrown> {
    if tag.get("id").and_then(Tag::as_str) != Some(ENTITY) {
        return None;
    }
    let triple = |key: &str| -> Option<DVec3> {
        let list = tag.get(key)?.as_list()?;
        let at = |i: usize| list.get(i)?.as_f64();
        Some(DVec3::new(at(0)?, at(1)?, at(2)?))
    };
    let rotation = tag.get("Rotation").and_then(Tag::as_list);
    let angle = |i: usize| {
        rotation
            .and_then(|r| r.get(i))
            .and_then(Tag::as_f64)
            .map_or(0.0, |v| v as f32)
    };
    let owner = tag
        .get("Owner")
        .and_then(Tag::as_ints)
        .and_then(|ints| <[i32; 4]>::try_from(ints).ok())
        .map(uuid_from_ints);
    let left_owner = tag
        .get("LeftOwner")
        .and_then(Tag::as_i64)
        .is_some_and(|v| v != 0);
    Thrown::resumed(
        ENTITY,
        owner,
        triple("Pos")?,
        triple("Motion").unwrap_or(DVec3::ZERO),
        angle(0),
        angle(1),
        left_owner,
        random,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shot(random: &mut LegacyRandom) -> Thrown {
        Thrown::shot_by_mob(
            ENTITY,
            7,
            DVec3::new(1.5, 65.185, 2.5),
            DVec3::new(0.0, 0.0, 4.0),
            0.7,
            0.0,
            random,
        )
        .unwrap()
    }

    #[test]
    fn a_breeze_charge_in_flight_saves_and_loads_with_its_owner() {
        let mut random = LegacyRandom::new(3);
        let charge = shot(&mut random);
        assert!(is_charge(&charge));
        let tag = to_tag(&charge, Some(0xB0B)).unwrap();
        assert_eq!(tag.get("id").and_then(Tag::as_str), Some(ENTITY));
        assert_eq!(
            tag.get("Owner").and_then(Tag::as_ints),
            Some(uuid_to_ints(0xB0B).to_vec())
        );
        assert_eq!(tag.get("HasBeenShot").and_then(Tag::as_i64), Some(1));
        let loaded = from_tag(&tag, &mut random).unwrap();
        assert!(is_charge(&loaded));
        assert_eq!(loaded.position(), charge.position());
        assert_eq!(loaded.velocity(), charge.velocity());
        assert_eq!(
            (loaded.yaw(), loaded.pitch()),
            (charge.yaw(), charge.pitch())
        );
        assert_eq!((loaded.shooter, loaded.shooter_uuid), (None, Some(0xB0B)));
        assert_eq!(loaded.owner, crate::thrown::MOB_THROWER);
        let ownerless = to_tag(&charge, None).unwrap();
        assert!(ownerless.get("Owner").is_none());
        assert_eq!(
            from_tag(&ownerless, &mut random).unwrap().shooter_uuid,
            None
        );
    }

    #[test]
    fn only_breeze_charges_are_saved() {
        let mut random = LegacyRandom::new(3);
        let thrown = Thrown::shot_by_mob(
            "minecraft:wind_charge",
            7,
            DVec3::ZERO,
            DVec3::Z,
            1.0,
            0.0,
            &mut random,
        )
        .unwrap();
        assert!(!is_charge(&thrown));
        assert!(to_tag(&thrown, None).is_none());
        let mut gone = shot(&mut random);
        gone.discard();
        assert!(to_tag(&gone, None).is_none());
        let other = minecraftoss_core::snbt::parse_compound(
            "{id:\"minecraft:wind_charge\",Pos:[0.0d,64.0d,0.0d]}",
        )
        .unwrap();
        assert!(from_tag(&other, &mut random).is_none());
    }
}
