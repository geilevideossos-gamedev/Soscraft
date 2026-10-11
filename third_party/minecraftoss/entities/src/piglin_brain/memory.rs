use crate::brain_behaviors::{Pos, Track, WalkGoal};
use crate::monster_ai::Target;
use crate::villager_brain::Slot;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GlobalPos {
    pub dimension: String,
    pub pos: Pos,
}

#[derive(Clone, Debug, Default)]
pub struct Memories {
    pub nearest_living: Slot<Vec<Target>>,
    pub visible_living: Slot<Vec<Target>>,
    pub nearest_players: Slot<Vec<Target>>,
    pub nearest_visible_player: Slot<Target>,
    pub nearest_visible_attackable_player: Slot<Target>,
    pub nearest_player_holding_wanted_item: Slot<Target>,
    pub nearest_repellent: Slot<Pos>,
    pub look_target: Slot<Track>,
    pub walk_target: Slot<WalkGoal>,
    pub cant_reach_walk_target_since: Slot<i64>,
    pub interaction_target: Slot<Target>,
    pub attack_target: Slot<Target>,
    pub avoid_target: Slot<Target>,
    pub celebrate_location: Slot<Pos>,
    pub dancing: Slot<()>,
    pub admiring_item: Slot<()>,
    pub admiring_disabled: Slot<()>,
    pub hunted_recently: Slot<()>,
    pub angry_at: Slot<u128>,
    pub universal_anger: Slot<()>,
    pub home: Slot<GlobalPos>,
    pub nearest_visible_nemesis: Slot<Target>,
    pub nearest_targetable_player_not_wearing_gold: Slot<Target>,
    pub nearest_visible_adult_piglins: Slot<Vec<Target>>,
    pub visible_adult_piglin_count: Slot<usize>,
    pub nearby_adult_piglins: Slot<Vec<Target>>,
    pub nearest_visible_zombified: Slot<Target>,
    pub attack_cooling_down: Slot<()>,
}

impl Memories {
    pub fn tick(&mut self) {
        self.nearest_living.tick();
        self.visible_living.tick();
        self.nearest_players.tick();
        self.nearest_visible_player.tick();
        self.nearest_visible_attackable_player.tick();
        self.nearest_player_holding_wanted_item.tick();
        self.nearest_repellent.tick();
        self.look_target.tick();
        self.walk_target.tick();
        self.cant_reach_walk_target_since.tick();
        self.interaction_target.tick();
        self.attack_target.tick();
        self.avoid_target.tick();
        self.celebrate_location.tick();
        self.dancing.tick();
        self.admiring_item.tick();
        self.admiring_disabled.tick();
        self.hunted_recently.tick();
        self.angry_at.tick();
        self.universal_anger.tick();
        self.home.tick();
        self.nearest_visible_nemesis.tick();
        self.nearest_targetable_player_not_wearing_gold.tick();
        self.nearest_visible_adult_piglins.tick();
        self.visible_adult_piglin_count.tick();
        self.nearby_adult_piglins.tick();
        self.nearest_visible_zombified.tick();
        self.attack_cooling_down.tick();
    }

    pub fn nearby_adult_piglins(&self) -> Vec<Target> {
        self.nearby_adult_piglins.get().cloned().unwrap_or_default()
    }

    pub fn sees(&self, target: Target) -> bool {
        self.visible_living
            .get()
            .is_some_and(|visible| visible.contains(&target))
    }
}
