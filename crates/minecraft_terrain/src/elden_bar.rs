use glam::DVec3;
use minecraftoss_entities::world::EntityWorld;

pub const COLOR: &str = "red";
pub const OVERLAY: &str = "progress";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EldenBar {
    pub id: u64,
    pub name: &'static str,
    pub progress: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShownBoss {
    pub bar: EldenBar,
    pub position: DVec3,
    pub range: f64,
}

pub fn bosses(world: &EntityWorld) -> Vec<ShownBoss> {
    world
        .elden_bosses()
        .iter()
        .filter(|mob| !mob.core.removed && mob.state.brain.awake())
        .map(|mob| {
            let brain = &mob.state.brain;
            ShownBoss {
                bar: EldenBar {
                    id: mob.core.id,
                    name: mob.state.def.name,
                    progress: progress(brain.health(), brain.max_health()),
                },
                position: mob.core.body.position,
                range: mob.state.def.arena.bar,
            }
        })
        .collect()
}

pub fn progress(health: f32, max: f32) -> f32 {
    if max > 0.0 {
        (health / max).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

pub fn bars_for(bosses: &[ShownBoss], feet: [f64; 3]) -> Vec<EldenBar> {
    let feet = DVec3::from_array(feet);
    bosses
        .iter()
        .filter(|boss| boss.position.distance_squared(feet) <= boss.range * boss.range)
        .map(|boss| boss.bar)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use elden_boss::Source;
    use elden_boss::margit::MARGIT;
    use minecraftoss_entities::tempt::PlayerCandidate;
    use minecraftoss_player::{Block, Pos, World};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    fn player_at(id: u64, position: DVec3) -> PlayerCandidate {
        PlayerCandidate {
            id,
            position,
            eye_height: 1.62,
            main_hand_cow_food: false,
            offhand_cow_food: false,
            main_hand_pig_food: false,
            offhand_pig_food: false,
            main_hand_chicken_food: false,
            offhand_chicken_food: false,
            main_hand_carrot_on_a_stick: false,
            offhand_carrot_on_a_stick: false,
            main_hand_wolf_interest: false,
            offhand_wolf_interest: false,
            main_hand_horse_tempt: false,
            offhand_horse_tempt: false,
            alive: true,
            spectator: false,
            attackable: true,
        }
    }

    fn woken() -> (EntityWorld, u64) {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        let id = world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        let players = [player_at(1, DVec3::new(0.5, 64.0, 10.0))];
        for _ in 0..2 {
            world.tick_with_players(&mut Field, &players);
        }
        assert!(world.elden_bosses()[0].state.brain.awake());
        (world, id)
    }

    #[test]
    fn a_player_within_48_blocks_of_an_awake_margit_gets_his_bar() {
        let (mut world, id) = woken();
        let shown = bosses(&world);
        let at = world.elden_bosses()[0].core.body.position;
        let away = |dx: f64, dy: f64| (at + DVec3::new(dx, dy, 0.0)).to_array();
        let bar = EldenBar {
            id,
            name: "Margit, the Fell Omen",
            progress: 1.0,
        };
        assert_eq!(bars_for(&shown, away(47.5, 0.0)), vec![bar]);
        assert_eq!(bars_for(&shown, away(0.0, -47.5)), vec![bar]);
        assert!(bars_for(&shown, away(48.5, 0.0)).is_empty());
        assert!(
            bars_for(&shown, away(34.0, 34.0)).is_empty(),
            "48 blocks every way, up and down too"
        );
        world
            .elden_boss_mut(id)
            .unwrap()
            .state
            .brain
            .set_health(200.0);
        let hurt = bars_for(&bosses(&world), away(3.0, 0.0));
        assert_eq!(hurt[0].progress, 0.25, "health over his maximum");
    }

    #[test]
    fn an_asleep_margit_shows_no_bar() {
        let mut world = EntityWorld::default();
        let at = DVec3::new(0.5, 64.0, 0.5);
        world.spawn_elden_boss(&MARGIT, at, 0.0, at, false);
        assert!(bosses(&world).is_empty(), "asleep");
        let (mut world, id) = woken();
        world.elden_boss_mut(id).unwrap().state.brain.reset(0);
        assert!(bosses(&world).is_empty(), "walking home after a reset");
        let (mut world, id) = woken();
        let killed = world.elden_hurt(id, Source::Environment { damage: 1.0e4 }, None);
        assert!(killed.is_some_and(|result| result.applied));
        assert_eq!(world.elden_bosses().len(), 1, "his body is still there");
        assert!(bosses(&world).is_empty(), "dead");
    }

    #[test]
    fn a_player_in_another_level_gets_no_bar() {
        let (world, _) = woken();
        let other = EntityWorld::default();
        let feet = world.elden_bosses()[0].core.body.position.to_array();
        assert_eq!(bars_for(&bosses(&world), feet).len(), 1);
        assert!(
            bars_for(&bosses(&other), feet).is_empty(),
            "each level shows its own bosses"
        );
    }
}
