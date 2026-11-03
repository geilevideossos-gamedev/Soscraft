use crate::families::{TerrainFamily, kinds, tracking_range};
use minecraftoss_entities::world::families::Family;
use minecraftoss_entities::world::families::pests::endermites::Endermite;
use minecraftoss_entities::world::families::pests::silverfish::Silverfish;
use minecraftoss_entities::world::families::trial::breezes::Breeze;

#[test]
fn a_family_s_tracking_range_by_kind_defaults_to_its_constant() {
    for kind in Silverfish::KINDS {
        assert_eq!(<Silverfish as TerrainFamily>::tracking_range(kind), 8);
    }
    for kind in Endermite::KINDS {
        assert_eq!(<Endermite as TerrainFamily>::tracking_range(kind), 8);
    }
    for kind in Breeze::KINDS {
        assert_eq!(<Breeze as TerrainFamily>::tracking_range(kind), 10);
    }
    assert_eq!(
        (
            tracking_range("minecraft:silverfish"),
            tracking_range("silverfish")
        ),
        (Some(8), Some(8))
    );
    assert_eq!(
        (tracking_range("minecraft:breeze"), tracking_range("breeze")),
        (Some(10), Some(10))
    );
    for kind in kinds() {
        let range = tracking_range(kind).unwrap_or_else(|| panic!("{kind} has a range"));
        assert_eq!(crate::mob_tracking::tracking_range(kind), range, "{kind}");
        if let Some(short) = kind.strip_prefix("minecraft:") {
            assert_eq!(tracking_range(short), Some(range), "{kind}");
            assert_eq!(crate::mob_tracking::tracking_range(short), range, "{kind}");
        }
    }
    assert_eq!(tracking_range("minecraft:zombie"), None);
    assert_eq!(tracking_range("minecraft:not_a_mob"), None);
    assert_eq!(crate::mob_tracking::tracking_range("minecraft:zombie"), 8);
}
