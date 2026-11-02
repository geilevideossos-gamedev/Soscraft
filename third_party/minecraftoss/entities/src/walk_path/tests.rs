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
