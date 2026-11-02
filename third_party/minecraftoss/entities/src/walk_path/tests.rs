use super::*;
use minecraftoss_player::Block;
use std::collections::BTreeMap;

#[derive(Default)]
struct Scene(BTreeMap<NodePos, Block>);

impl World for Scene {
    fn block(&self, pos: NodePos) -> Option<Block> {
        self.0.get(&pos).cloned()
    }

    fn set_block(&mut self, pos: NodePos, block: Option<Block>) {
        match block {
            Some(block) => self.0.insert(pos, block),
            None => self.0.remove(&pos),
        };
    }
}

impl Scene {
    fn fill(&mut self, low: NodePos, high: NodePos, id: &str) {
        for x in low.0..=high.0 {
            for y in low.1..=high.1 {
                for z in low.2..=high.2 {
                    self.0.insert((x, y, z), Block::new(id));
                }
            }
        }
    }
}

fn walker(position: DVec3, in_floatable_fluid: bool) -> Walker {
    Walker {
        position,
        on_ground: !in_floatable_fluid,
        in_floatable_fluid,
    }
}

fn walled_field() -> Scene {
    let mut scene = Scene::default();
    scene.fill((-8, 63, -8), (16, 63, 8), "minecraft:stone");
    scene.fill((5, 64, -4), (5, 65, 4), "minecraft:stone");
    scene.fill((8, 62, -2), (11, 62, 2), "minecraft:stone");
    scene.fill((8, 63, -2), (11, 63, 2), "minecraft:water");
    scene
}

#[test]
fn the_walk_search_is_unchanged_behind_the_evaluator_split() {
    let scene = walled_field();
    let cow = WalkProfile::animal(0.9, 1.4);
    let from = DVec3::new(0.5, 64.0, 0.5);
    let found = |path: FoundPath| (path.nodes, path.reached, path.target);
    let around = find_walk_path(
        &scene,
        &cow,
        walker(from, false),
        DVec3::new(12.5, 64.0, 6.5),
        0,
    );
    let nodes = vec![
        (0, 64, 0),
        (1, 64, 1),
        (2, 64, 2),
        (3, 64, 3),
        (4, 64, 4),
        (4, 64, 5),
        (5, 64, 5),
        (6, 64, 5),
        (7, 64, 5),
        (8, 64, 5),
        (9, 64, 5),
        (10, 64, 5),
        (11, 64, 6),
        (12, 64, 6),
    ];
    assert_eq!(found(around), (nodes, true, (12, 64, 6)));
    let targets = [(14, 64, 0), (12, 64, -6), (2, 64, 7)];
    let any = find_walk_path_to_any(&scene, &cow, walker(from, false), &targets, 1);
    let nodes = vec![
        (0, 64, 0),
        (0, 64, 1),
        (1, 64, 2),
        (1, 64, 3),
        (1, 64, 4),
        (1, 64, 5),
        (2, 64, 6),
    ];
    assert_eq!(any.map(found), Some((nodes, true, (2, 64, 7))));
    let afloat = DVec3::new(9.5, 63.1, 0.5);
    let out = find_walk_path(
        &scene,
        &cow,
        walker(afloat, true),
        DVec3::new(14.5, 64.0, 0.5),
        0,
    );
    let nodes = vec![
        (9, 63, 0),
        (10, 63, 0),
        (11, 63, 0),
        (12, 64, 0),
        (13, 64, 0),
        (14, 64, 0),
    ];
    assert_eq!(found(out), (nodes, true, (14, 64, 0)));
    let mut sinker = WalkProfile::animal(0.6, 1.95);
    sinker.can_float = false;
    let sunk = find_walk_path(
        &scene,
        &sinker,
        walker(from, false),
        DVec3::new(10.5, 63.0, 0.5),
        0,
    );
    let nodes = vec![(0, 64, 0), (1, 64, 0), (2, 64, 0), (3, 64, 0), (4, 64, 0)];
    assert_eq!(found(sunk), (nodes, false, (10, 63, 0)));
    let short = find_walk_path(
        &scene,
        &cow,
        walker(from, false),
        DVec3::new(15.5, 64.0, -7.5),
        0,
    );
    let nodes = vec![
        (0, 64, 0),
        (1, 64, -1),
        (2, 64, -2),
        (3, 64, -3),
        (4, 64, -4),
        (4, 64, -5),
        (5, 64, -5),
        (6, 64, -5),
        (7, 64, -6),
        (8, 64, -6),
        (9, 64, -6),
        (10, 64, -7),
        (11, 64, -7),
        (12, 64, -8),
    ];
    assert_eq!(found(short), (nodes, false, (15, 64, -8)));
    let mut flyer = WalkProfile::animal(0.5, 0.9);
    flyer.flying = Some(crate::flying::Flight::default());
    flyer.max_visited = Some(768);
    flyer.max_path_length = Some(48.0);
    let mut random = LegacyRandom::new(7);
    let flown = find_fly_path_to_any(
        &scene,
        &flyer,
        walker(from, false),
        &[(10, 64, 0)],
        0,
        &mut random,
    );
    let nodes = vec![
        (0, 64, 0),
        (1, 65, 0),
        (2, 65, 0),
        (3, 65, 0),
        (4, 66, 0),
        (5, 67, 0),
        (6, 66, 0),
        (7, 65, 0),
        (8, 65, 0),
        (9, 65, 0),
        (10, 64, 0),
    ];
    assert_eq!(flown.map(found), Some((nodes, true, (10, 64, 0))));
}

fn swimmer(width: f32, height: f32) -> WalkProfile {
    let mut profile = WalkProfile::animal(width, height);
    profile.set_malus(PathType::Water, 0.0);
    profile
}

fn swim(
    scene: &Scene,
    profile: &WalkProfile,
    from: DVec3,
    to: NodePos,
    breaching: bool,
) -> FoundPath {
    find_swim_path_to_any(scene, profile, walker(from, true), &[to], 0, breaching)
        .expect("a swim path")
}

fn steps_are_faces_or_level_diagonals(nodes: &[NodePos]) {
    for pair in nodes.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let step = ((b.0 - a.0).abs(), (b.1 - a.1).abs(), (b.2 - a.2).abs());
        let face = step.0 + step.1 + step.2 == 1;
        let diagonal = step == (1, 0, 1);
        assert!(face || diagonal, "{a:?} to {b:?}");
    }
}

fn walled_pool() -> Scene {
    let mut scene = Scene::default();
    scene.fill((-2, 58, -6), (12, 58, 6), "minecraft:stone");
    scene.fill((-2, 59, -6), (12, 60, 6), "minecraft:water");
    scene.fill((4, 59, -6), (4, 60, 1), "minecraft:glass");
    scene
}

#[test]
fn a_swim_path_goes_around_a_glass_wall_through_water_only() {
    let scene = walled_pool();
    let fish = swimmer(0.5, 0.3);
    let from = DVec3::new(0.5, 60.0, 0.5);
    let path = swim(&scene, &fish, from, (8, 60, 0), false);
    assert!(path.reached, "{:?}", path.nodes);
    assert_eq!(path.nodes.first(), Some(&(0, 60, 0)));
    assert_eq!(path.nodes.last(), Some(&(8, 60, 0)));
    assert!(path.nodes.iter().all(|&node| scene.floatable_fluid(node)));
    assert!(path.nodes.iter().all(|&(x, _, z)| x != 4 || z >= 2));
    assert!(path.nodes.iter().any(|&(x, _, _)| x == 4));
    steps_are_faces_or_level_diagonals(&path.nodes);
    let diagonal = |p: &[NodePos]| (p[1].0 - p[0].0).abs() + (p[1].2 - p[0].2).abs() == 2;
    assert!(path.nodes.windows(2).any(diagonal), "{:?}", path.nodes);
    let mut sealed = walled_pool();
    sealed.fill((4, 59, 2), (4, 60, 6), "minecraft:glass");
    let path = swim(&sealed, &fish, from, (8, 60, 0), false);
    assert!(!path.reached);
    assert!(path.nodes.iter().all(|&(x, _, _)| x < 4));
    let mut fenced = walled_pool();
    for z in 2..=6 {
        for y in 59..=60 {
            let fence = Block::new("minecraft:oak_fence").with("waterlogged", "true");
            fenced.0.insert((4, y, z), fence);
        }
    }
    assert!(fenced.floatable_fluid((4, 60, 2)));
    assert!(!pathfindable_in_water(&fenced, (4, 60, 2)));
    assert_eq!(
        swim_path_type(&fenced, (4, 60, 2), (1, 1, 1)),
        PathType::Blocked
    );
    let path = swim(&fenced, &fish, from, (8, 60, 0), false);
    assert!(!path.reached);
    let slab = Block::new("minecraft:stone_slab").with("waterlogged", "true");
    fenced.0.insert((4, 60, 5), slab);
    assert!(pathfindable_in_water(&fenced, (4, 60, 5)));
    let trapdoor = Block::new("minecraft:oak_trapdoor").with("waterlogged", "true");
    fenced.0.insert((4, 61, 5), trapdoor);
    assert!(pathfindable_in_water(&fenced, (4, 61, 5)));
    assert!(!pathfindable_in_water(&fenced, (4, 64, 5)));
    fenced
        .0
        .insert((4, 64, 5), Block::new("minecraft:powder_snow"));
    assert!(pathfindable_in_water(&fenced, (4, 64, 5)));
}

fn channel() -> Scene {
    let mut scene = Scene::default();
    scene.fill((-2, 58, -2), (12, 60, 2), "minecraft:stone");
    scene.fill((-2, 59, -1), (12, 60, 1), "minecraft:water");
    scene.fill((10, 59, 1), (10, 60, 1), "minecraft:glass");
    scene
}

#[test]
fn a_swim_path_breaches_only_for_a_dolphin() {
    let scene = channel();
    let swimmer = swimmer(0.9, 0.6);
    let from = DVec3::new(0.5, 60.0, 0.5);
    let fish = swim(&scene, &swimmer, from, (8, 61, 0), false);
    assert!(!fish.reached);
    assert!(fish.nodes.iter().all(|&node| scene.floatable_fluid(node)));
    let dolphin = swim(&scene, &swimmer, from, (8, 61, 0), true);
    assert!(dolphin.reached, "{:?}", dolphin.nodes);
    assert_eq!(dolphin.nodes.last(), Some(&(8, 61, 0)));
    let (last, rest) = dolphin.nodes.split_last().expect("nodes");
    assert!(!scene.floatable_fluid(*last));
    assert!(rest.len() >= 8, "{:?}", dolphin.nodes);
    assert!(rest.iter().all(|&node| scene.floatable_fluid(node)));
    let size = (1, 1, 1);
    assert_eq!(swim_path_type(&scene, (7, 61, 0), size), PathType::Breach);
    assert_eq!(swim_path_type(&scene, (7, 62, 0), size), PathType::Blocked);
    assert_eq!(swim_path_type(&scene, (10, 61, 1), size), PathType::Blocked);
    assert_eq!(swim_path_type(&scene, (7, 60, 0), size), PathType::Water);
    assert_eq!(swim_path_type(&scene, (10, 60, 1), size), PathType::Blocked);
    let walk = Evaluator::new(&scene, &swimmer, walker(from, true));
    let mut evaluator = SwimEvaluator {
        walk,
        breaching: true,
    };
    let air = evaluator.accepted((7, 61, 0)).expect("a breach node");
    assert_eq!(evaluator.walk.nodes[air].cost_malus, 4.0 + 8.0);
    assert_eq!(evaluator.walk.nodes[air].path_type, Some(PathType::Breach));
    evaluator.accepted((7, 61, 0));
    assert_eq!(evaluator.walk.nodes[air].cost_malus, 12.0 + 8.0);
    let water = evaluator.accepted((7, 60, 0)).expect("a water node");
    assert_eq!(evaluator.walk.nodes[water].cost_malus, 0.0);
    let middle = evaluator.walk.node((3, 60, 0));
    let around = evaluator.neighbors(middle);
    let positions: Vec<NodePos> = around
        .iter()
        .map(|&n| evaluator.walk.nodes[n].pos)
        .collect();
    let expected = [
        (3, 59, 0),
        (3, 61, 0),
        (3, 60, -1),
        (3, 60, 1),
        (2, 60, 0),
        (4, 60, 0),
        (4, 60, -1),
        (4, 60, 1),
        (2, 60, 1),
        (2, 60, -1),
    ];
    assert_eq!(positions, expected);
    evaluator.breaching = false;
    assert_eq!(evaluator.accepted((6, 61, 0)), None);
}

#[test]
fn a_two_block_swimmer_does_not_fit_a_one_block_tunnel() {
    let mut scene = Scene::default();
    scene.fill((-1, 57, -3), (13, 63, 3), "minecraft:stone");
    scene.fill((-1, 58, -3), (2, 62, 3), "minecraft:water");
    scene.fill((10, 58, -3), (13, 62, 3), "minecraft:water");
    scene.fill((3, 60, 0), (9, 60, 0), "minecraft:water");
    let from = DVec3::new(0.5, 60.0, 0.5);
    let fish = swim(&scene, &swimmer(0.5, 0.3), from, (11, 60, 0), false);
    assert!(fish.reached);
    assert!((3..=9).all(|x| fish.nodes.contains(&(x, 60, 0))));
    let tall = swim(&scene, &swimmer(0.9, 1.2), from, (11, 60, 0), false);
    assert!(!tall.reached);
    assert!(tall.nodes.iter().all(|&(x, _, _)| x <= 2));
    let wide = swim(&scene, &swimmer(1.2, 0.5), from, (11, 60, 0), false);
    assert!(!wide.reached);
}

#[test]
fn the_swim_start_is_half_a_block_above_the_feet() {
    let scene = walled_pool();
    let fish = swimmer(0.5, 0.3);
    let start =
        |x: f64, y: f64| swim(&scene, &fish, DVec3::new(x, y, 0.5), (2, 60, 0), false).nodes[0];
    assert_eq!(start(2.5, 60.6), (2, 61, 0));
    assert_eq!(start(2.5, 60.4), (2, 60, 0));
    assert_eq!(start(2.1, 60.4), (1, 60, 0));
    assert_eq!(start(2.25, 59.5), (2, 60, 0));
    let near = swim(&scene, &fish, DVec3::new(2.5, 59.9, 0.5), (2, 60, 0), false);
    assert_eq!(near.nodes, vec![(2, 60, 0)]);
    assert!(near.reached);
}

fn pond() -> Scene {
    let mut scene = Scene::default();
    scene.fill((-4, 57, -4), (10, 63, 4), "minecraft:stone");
    scene.fill((0, 58, -1), (4, 63, 1), "minecraft:water");
    scene
}

#[test]
fn an_amphibious_path_swims_across_a_pond_and_walks_out() {
    let scene = pond();
    let mut axolotl = WalkProfile::animal(0.75, 0.42);
    axolotl.can_float = false;
    axolotl.amphibious = Some(false);
    let prepared = amphibious_prepared(&axolotl);
    assert_eq!(prepared.malus(PathType::Walkable), 6.0);
    assert_eq!(prepared.malus(PathType::WaterBorder), 4.0);
    assert_eq!(prepared.malus(PathType::Water), 0.0);
    assert_eq!(axolotl.malus(PathType::Walkable), 0.0);
    let from = DVec3::new(1.5, 61.0, 0.5);
    let path = find_walk_path(
        &scene,
        &axolotl,
        walker(from, true),
        DVec3::new(8.5, 64.0, 0.5),
        0,
    );
    assert!(path.reached);
    assert_eq!(path.nodes.first(), Some(&(1, 61, 0)));
    assert_eq!(path.nodes.last(), Some(&(8, 64, 0)));
    let out = path
        .nodes
        .iter()
        .position(|&node| !scene.floatable_fluid(node))
        .expect("out of the water");
    assert!(
        path.nodes[..out]
            .iter()
            .all(|&node| scene.floatable_fluid(node))
    );
    assert!(path.nodes[out..].iter().all(|&(x, y, _)| y == 64 && x >= 5));
    let mut walker_profile = WalkProfile::animal(0.75, 0.42);
    walker_profile.can_float = false;
    let sunk = find_walk_path(
        &scene,
        &walker_profile,
        walker(from, true),
        DVec3::new(8.5, 64.0, 0.5),
        0,
    );
    assert_ne!(sunk.nodes.first(), Some(&(1, 61, 0)));
    let mut evaluator = Evaluator::new(&scene, &prepared, walker(from, true));
    assert_eq!(evaluator.floor_level((2, 61, 0)), 61.5);
    assert_eq!(evaluator.cached_path_type((2, 61, 0)), PathType::Water);
    assert_eq!(
        evaluator.cached_path_type((2, 58, 0)),
        PathType::WaterBorder
    );
    assert_eq!(
        evaluator.cached_path_type((0, 61, 0)),
        PathType::WaterBorder
    );
    let middle = evaluator.node((2, 60, 0));
    let around = evaluator.neighbors(middle);
    let positions: Vec<NodePos> = around.iter().map(|&n| evaluator.nodes[n].pos).collect();
    assert!(positions.contains(&(2, 61, 0)), "{positions:?}");
    assert!(positions.contains(&(2, 59, 0)), "{positions:?}");
    let low = evaluator.node((2, 59, 0));
    let around = evaluator.neighbors(low);
    let positions: Vec<NodePos> = around.iter().map(|&n| evaluator.nodes[n].pos).collect();
    assert!(!positions.contains(&(2, 58, 0)), "{positions:?}");
    let mut deep = axolotl.clone();
    deep.amphibious = Some(true);
    deep.sea_level = 73;
    let deep = amphibious_prepared(&deep);
    let mut shallow_search = Evaluator::new(&scene, &deep, walker(from, true));
    let middle = shallow_search.node((2, 60, 0));
    let around = shallow_search.neighbors(middle);
    let costs: Vec<(NodePos, f32)> = around
        .iter()
        .map(|&n| {
            (
                shallow_search.nodes[n].pos,
                shallow_search.nodes[n].cost_malus,
            )
        })
        .collect();
    assert!(costs.contains(&((2, 61, 0), 1.0)), "{costs:?}");
    assert!(costs.contains(&((3, 60, 0), 1.0)), "{costs:?}");
    assert!(costs.contains(&((2, 60, 1), 4.0)), "{costs:?}");
    let mut level = deep.clone();
    level.sea_level = 63;
    let mut level_search = Evaluator::new(&scene, &level, walker(from, true));
    let middle = level_search.node((2, 60, 0));
    let around = level_search.neighbors(middle);
    let costs: Vec<(NodePos, f32)> = around
        .iter()
        .map(|&n| (level_search.nodes[n].pos, level_search.nodes[n].cost_malus))
        .collect();
    assert!(costs.contains(&((2, 61, 0), 0.0)), "{costs:?}");
    assert!(costs.contains(&((3, 60, 0), 0.0)), "{costs:?}");
}
