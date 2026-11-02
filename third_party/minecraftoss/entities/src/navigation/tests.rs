use super::*;
use minecraftoss_player::Block;
use std::collections::BTreeMap;

#[derive(Default)]
struct Scene(BTreeMap<(i32, i32, i32), Block>);

impl World for Scene {
    fn block(&self, pos: (i32, i32, i32)) -> Option<Block> {
        self.0.get(&pos).cloned()
    }

    fn set_block(&mut self, pos: (i32, i32, i32), block: Option<Block>) {
        match block {
            Some(block) => self.0.insert(pos, block),
            None => self.0.remove(&pos),
        };
    }
}

impl Scene {
    fn fill(&mut self, low: (i32, i32, i32), high: (i32, i32, i32), id: &str) {
        for x in low.0..=high.0 {
            for y in low.1..=high.1 {
                for z in low.2..=high.2 {
                    self.0.insert((x, y, z), Block::new(id));
                }
            }
        }
    }
}

const BOUND: WaterNav = WaterNav::Bound { breaching: false };
const BREACHING: WaterNav = WaterNav::Bound { breaching: true };
const AMPHIBIOUS: WaterNav = WaterNav::Amphibious { shallow: false };

fn pool() -> Scene {
    let mut scene = Scene::default();
    scene.fill((-6, 58, -6), (6, 58, 6), "minecraft:stone");
    scene.fill((-6, 59, -6), (6, 63, 6), "minecraft:water");
    scene
}

fn in_water() -> FluidFrame {
    FluidFrame {
        water_height: 1.0,
        lava_height: 0.0,
    }
}

fn following(water: WaterNav, nodes: Vec<(i32, i32, i32)>) -> GroundNavigation {
    let mut navigation = GroundNavigation {
        water,
        ..GroundNavigation::default()
    };
    assert!(navigation.follow(nodes, 1.0, DVec3::ZERO));
    navigation
}

fn swimmer() -> WalkProfile {
    let mut profile = WalkProfile::animal(0.5, 0.3);
    profile.set_malus(minecraftoss_player::path_type::PathType::Water, 0.0);
    profile
}

#[test]
fn a_water_bound_mob_reaches_a_node_within_half_a_block_up_or_down() {
    let scene = pool();
    let at = |y: f64| Body::new(DVec3::new(0.5, y, 0.5), 0.5, 0.3);
    let cases = [
        (BOUND, 61.6, false),
        (BOUND, 61.4, true),
        (BOUND, 60.6, true),
        (BOUND, 60.4, false),
        (AMPHIBIOUS, 61.9, true),
        (AMPHIBIOUS, 60.1, true),
        (AMPHIBIOUS, 62.1, false),
    ];
    for (water, y, reached) in cases {
        let mut navigation = following(water, vec![(0, 61, 0)]);
        let wanted = navigation.tick_water(&scene, &at(y), in_water(), 0.7);
        if reached {
            assert_eq!(wanted, None, "{water:?} at {y}");
            assert!(navigation.is_done());
        } else {
            let at_node = DVec3::new(0.5, 61.0, 0.5);
            assert_eq!(wanted, Some((at_node, 1.0)), "{water:?} at {y}");
        }
    }
    assert_eq!(ground_y(&scene, DVec3::new(0.5, 61.0, 0.5)), 60.0);
}

#[test]
fn a_stranded_swimmer_makes_no_path_unless_it_breaches() {
    let mut scene = Scene::default();
    scene.fill((-6, 63, -6), (6, 63, 6), "minecraft:stone");
    scene.fill((2, 63, -1), (4, 63, 1), "minecraft:water");
    let mut body = Body::new(DVec3::new(0.5, 64.0, 0.5), 0.5, 0.3);
    body.on_ground = true;
    let profile = swimmer();
    let dry = FluidFrame::default();
    let target = DVec3::new(3.5, 63.0, 0.5);
    let mut fish = following(BOUND, vec![(0, 64, 0), (1, 64, 0)]);
    fish.stop();
    assert_eq!(
        plan_walk_path(&body, &mut fish, &scene, &profile, dry, target, 0),
        None
    );
    assert_eq!(
        navigate_walk_to(&body, &mut fish, &scene, &profile, dry, target, 1.0, 0),
        Some(false)
    );
    let mut dolphin = GroundNavigation {
        water: BREACHING,
        ..GroundNavigation::default()
    };
    let planned = plan_walk_path(&body, &mut dolphin, &scene, &profile, dry, target, 0)
        .expect("a breacher plans out of water");
    assert_eq!(planned.nodes(), &[(0, 64, 0)]);
    assert!(!planned.reached());
    assert_eq!(dolphin.target_pos, Some((3, 63, 0)));
    let mut wet = following(BOUND, vec![(0, 64, 0)]);
    wet.stop();
    let in_pond = Body::new(DVec3::new(2.5, 63.0, 0.5), 0.5, 0.3);
    let planned = plan_walk_path(&in_pond, &mut wet, &scene, &profile, in_water(), target, 0)
        .expect("a fish in water plans");
    assert!(planned.reached());
    let mut stranded = following(BOUND, vec![(0, 63, 0), (1, 63, 0)]);
    let mut falling = Body::new(DVec3::new(0.5, 64.2, 0.5), 0.5, 0.3);
    assert!(stranded.tick_water(&scene, &body, dry, 0.7).is_some());
    assert_eq!(stranded.next, 0);
    stranded.tick_water(&scene, &falling, dry, 0.7);
    assert_eq!(stranded.next, 1);
    falling.position.x = 2.5;
    falling.position.y = 63.5;
    stranded.tick_water(&scene, &falling, dry, 0.7);
    assert_eq!(stranded.next, 1);
}

#[test]
fn it_cuts_a_corner_only_with_a_clear_line() {
    let mut scene = pool();
    let nodes = vec![(1, 61, 1), (2, 61, 1)];
    let body = Body::new(DVec3::new(0.3, 61.0, 0.5), 0.5, 0.3);
    let mut navigation = following(BOUND, nodes.clone());
    navigation.tick_water(&scene, &body, in_water(), 0.7);
    assert_eq!(navigation.next, 1);
    let mut navigation = following(AMPHIBIOUS, nodes.clone());
    navigation.tick_water(&scene, &body, in_water(), 0.7);
    assert_eq!(navigation.next, 1);
    let mut navigation = following(AMPHIBIOUS, nodes.clone());
    navigation.tick_water(&scene, &body, FluidFrame::default(), 0.7);
    assert_eq!(navigation.next, 0);
    for wall in ["minecraft:stone", "minecraft:glass"] {
        scene.0.insert((0, 61, 1), Block::new(wall));
        let mut navigation = following(BOUND, nodes.clone());
        navigation.tick_water(&scene, &body, in_water(), 0.7);
        assert_eq!(navigation.next, 0, "{wall}");
    }
}

#[test]
fn a_water_navigation_follows_its_own_rules_to_update_its_path() {
    let scene = pool();
    let mut body = Body::new(DVec3::new(0.5, 66.0, 0.5), 0.5, 0.3);
    let profile = swimmer();
    let dry = FluidFrame::default();
    let target = DVec3::new(3.5, 61.0, 0.5);
    let mut ground = GroundNavigation::default();
    assert_eq!(
        plan_walk_path(&body, &mut ground, &scene, &profile, dry, target, 0),
        None
    );
    let mut amphibious = GroundNavigation {
        water: AMPHIBIOUS,
        ..GroundNavigation::default()
    };
    assert!(plan_walk_path(&body, &mut amphibious, &scene, &profile, dry, target, 0).is_some());
    let mut fish = GroundNavigation {
        water: BOUND,
        ..GroundNavigation::default()
    };
    body.position.y = 61.0;
    let planned = plan_walk_path(&body, &mut fish, &scene, &profile, in_water(), target, 0)
        .expect("a fish in water plans");
    assert!(planned.reached());
    assert_eq!(planned.nodes().first(), Some(&(0, 61, 0)));
    assert_eq!(planned.nodes().last(), Some(&(3, 61, 0)));
    assert!(fish.move_to(Some(planned), 1.0, body.position));
    assert!(!fish.recompute(&scene, &body, &profile, in_water(), 10));
    assert!(fish.recompute(&scene, &body, &profile, in_water(), 30));
    assert_eq!(fish.nodes.last(), Some(&(3, 61, 0)));
    body.position.y = 66.0;
    assert!(!fish.recompute(&scene, &body, &profile, dry, 60));
    let mut dolphin = fish.clone();
    dolphin.water = BREACHING;
    assert!(dolphin.recompute(&scene, &body, &profile, dry, 90));
}
