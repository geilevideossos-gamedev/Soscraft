use minecraftoss_core::nbt::Tag;
use minecraftoss_world::natural_spawner::spawning::MobCategory;

const ALLOWED_IN_PEACEFUL: [&str; 7] = [
    "minecraft:camel_husk",
    "minecraft:ender_dragon",
    "minecraft:piglin",
    "minecraft:shulker",
    "minecraft:sulfur_cube",
    "minecraft:zombie_horse",
    "minecraft:zombie_nautilus",
];

const NEVER_FAR_AWAY: [&str; 4] = [
    "minecraft:shulker",
    "minecraft:warden",
    "minecraft:ender_dragon",
    "minecraft:wither",
];

fn monster(kind: &str) -> bool {
    MobCategory::of_type(kind) == Some(MobCategory::Monster)
}

pub fn allowed_in_peaceful(kind: &str) -> bool {
    !monster(kind) || ALLOWED_IN_PEACEFUL.contains(&kind)
}

pub fn removed_when_far_away(kind: &str, tag: &Tag) -> bool {
    if NEVER_FAR_AWAY.contains(&kind) {
        return false;
    }
    if kind == "minecraft:zombie_villager" {
        let int = |key: &str| tag.get(key).and_then(Tag::as_i64);
        let converting = int("ConversionTime").is_some_and(|t| t > -1);
        return !converting && int("Xp").unwrap_or(0) == 0;
    }
    true
}

pub fn despawns(
    kind: &str,
    persistent: bool,
    tag: &Tag,
    peaceful: bool,
    nearest_squared: Option<f64>,
) -> bool {
    if !monster(kind) {
        return false;
    }
    if peaceful && !allowed_in_peaceful(kind) {
        return true;
    }
    if persistent || !removed_when_far_away(kind, tag) {
        return false;
    }
    nearest_squared.is_some_and(|d| d > 128.0 * 128.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn tag(entries: &[(&str, i32)]) -> Tag {
        Tag::Compound(
            entries
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Tag::Int(*v)))
                .collect::<BTreeMap<_, _>>(),
        )
    }

    const FAR: Option<f64> = Some(200.0 * 200.0);
    const NEAR: Option<f64> = Some(100.0 * 100.0);

    #[test]
    fn shulkers_stay_far_from_players_and_in_peaceful() {
        let none = tag(&[]);
        assert!(!despawns("minecraft:shulker", false, &none, false, FAR));
        assert!(!despawns("minecraft:shulker", false, &none, true, FAR));
    }

    #[test]
    fn plain_monsters_go_beyond_128_blocks_and_in_peaceful() {
        let none = tag(&[]);
        assert!(despawns("minecraft:husk", false, &none, false, FAR));
        assert!(!despawns("minecraft:husk", false, &none, false, NEAR));
        assert!(!despawns("minecraft:husk", true, &none, false, FAR));
        assert!(despawns("minecraft:husk", true, &none, true, NEAR));
        assert!(!despawns("minecraft:husk", false, &none, false, None));
    }

    #[test]
    fn peaceful_keeps_the_vanilla_exceptions_but_far_away_still_removes_them() {
        let none = tag(&[]);
        assert!(!despawns("minecraft:piglin", false, &none, true, NEAR));
        assert!(!despawns(
            "minecraft:zombie_horse",
            false,
            &none,
            true,
            NEAR
        ));
        assert!(despawns("minecraft:piglin", false, &none, true, FAR));
    }

    #[test]
    fn a_curing_or_traded_zombie_villager_stays() {
        let plain = tag(&[("ConversionTime", -1), ("Xp", 0)]);
        let curing = tag(&[("ConversionTime", 3000), ("Xp", 0)]);
        let traded = tag(&[("ConversionTime", -1), ("Xp", 12)]);
        assert!(despawns(
            "minecraft:zombie_villager",
            false,
            &plain,
            false,
            FAR
        ));
        assert!(!despawns(
            "minecraft:zombie_villager",
            false,
            &curing,
            false,
            FAR
        ));
        assert!(!despawns(
            "minecraft:zombie_villager",
            false,
            &traded,
            false,
            FAR
        ));
    }

    #[test]
    fn other_categories_are_left_alone() {
        let none = tag(&[]);
        assert!(!despawns("minecraft:horse", false, &none, true, FAR));
        assert!(!despawns("minecraft:minecart", false, &none, true, FAR));
    }
}
