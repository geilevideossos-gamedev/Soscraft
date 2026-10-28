use glam::DVec3;
use minecraftoss_entities::world::EntityWorld;

pub const NAME: &str = "entity.minecraft.wither";
pub const COLOR: &str = "purple";
pub const OVERLAY: &str = "progress";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WitherBar {
    pub id: u64,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownWither {
    pub id: u64,
    pub position: DVec3,
    pub progress: f32,
}

pub fn withers(world: &EntityWorld) -> Vec<ShownWither> {
    world
        .withers()
        .iter()
        .map(|e| ShownWither {
            id: e.id,
            position: e.wither.body.position,
            progress: e.wither.boss_progress(),
        })
        .collect()
}

pub fn bars_for(withers: &[ShownWither], feet: [f64; 3], view_distance: i32) -> Vec<WitherBar> {
    let range = crate::mob_tracking::tracking_range(minecraftoss_entities::wither::TYPE_ID);
    withers
        .iter()
        .filter(|w| crate::mob_tracking::tracks(range, view_distance, feet, w.position))
        .map(|w| WitherBar {
            id: w.id,
            progress: w.progress,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_entities::wither::Wither;

    #[test]
    fn a_wither_shows_its_bar_to_the_players_that_track_it() {
        let mut world = EntityWorld::default();
        let built = world.spawn_wither(Wither::built(DVec3::new(0.5, 64.0, 0.5), 0.0), true);
        let hatched = world.spawn_wither(Wither::new(DVec3::new(300.5, 64.0, 0.5)), true);
        world.wither_mut(hatched).unwrap().wither.health = 150.0;
        let shown = withers(&world);
        assert_eq!(shown.len(), 2);
        assert_eq!(shown[0].progress, 0.0, "a built wither's bar starts empty");
        assert_eq!(shown[1].progress, 0.5);
        let near = bars_for(&shown, [100.5, 64.0, 0.5], 12);
        assert_eq!(
            near,
            vec![WitherBar {
                id: built,
                progress: 0.0
            }]
        );
        assert!(
            bars_for(&shown, [100.5, 64.0, 0.5], 6).is_empty(),
            "a short view distance tracks it no farther"
        );
        let both = bars_for(&shown, [150.5, 64.0, 0.5], 10);
        assert_eq!(both.len(), 2, "ten chunks each way");
        world.wither_mut(built).unwrap().wither.invulnerable_ticks = 110;
        assert_eq!(withers(&world)[0].progress, 0.5, "it fills as it charges");
    }
}
