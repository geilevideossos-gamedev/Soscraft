use super::*;
use minecraftoss_player::Block;
use minecraftoss_player::path_type::PathType;
use std::collections::BTreeMap;

#[derive(Default)]
struct Scene(BTreeMap<BlockPos, Block>);

impl World for Scene {
    fn block(&self, pos: BlockPos) -> Option<Block> {
        self.0.get(&pos).cloned()
    }

    fn set_block(&mut self, pos: BlockPos, block: Option<Block>) {
        match block {
            Some(block) => self.0.insert(pos, block),
            None => self.0.remove(&pos),
        };
    }
}

impl Scene {
    fn fill(&mut self, low: BlockPos, high: BlockPos, id: &str) {
        for x in low.0..=high.0 {
            for y in low.1..=high.1 {
                for z in low.2..=high.2 {
                    self.0.insert((x, y, z), Block::new(id));
                }
            }
        }
    }
}

fn swimmer() -> WalkProfile {
    let mut profile = WalkProfile::animal(0.5, 0.3);
    profile.set_malus(PathType::Water, 0.0);
    profile.walk_target = WalkTarget::Zero;
    profile
}

fn block_of(spot: DVec3) -> BlockPos {
    (
        spot.x.floor() as i32,
        spot.y.floor() as i32,
        spot.z.floor() as i32,
    )
}

#[test]
fn a_water_bound_spot_needs_no_floor_but_no_solid_block() {
    let mut scene = Scene::default();
    scene.fill((-12, 40, -12), (12, 63, 12), "minecraft:water");
    scene.fill((2, 40, -12), (5, 63, 12), "minecraft:stone");
    let position = DVec3::new(0.5, 50.0, 0.5);
    let mut profile = swimmer();
    profile.set_malus(PathType::Blocked, 0.0);
    let (mut found, mut in_water, mut above) = (0, 0, 0);
    for seed in 0..200 {
        let mut random = LegacyRandom::new(seed);
        let spot =
            water_bound_random_position(&scene, &profile, position, 10, 7, None, &mut random);
        let Some(spot) = spot else { continue };
        found += 1;
        let block = block_of(spot);
        assert!(!scene.solid_render(block), "{spot:?}");
        assert!(!(2..=5).contains(&block.0), "{spot:?}");
        assert_eq!((spot.x.fract().abs(), spot.z.fract().abs()), (0.5, 0.5));
        if scene.floatable_fluid(block) {
            in_water += 1;
        } else {
            above += 1;
        }
        let mut random = LegacyRandom::new(seed);
        assert_eq!(
            default_random_position(&scene, &swimmer(), position, 10, 7, &mut random),
            None
        );
    }
    assert!(found > 150, "{found}");
    assert!(in_water > 100 && above == 0, "{in_water} {above}");
    let near_surface = DVec3::new(-6.5, 62.0, 0.5);
    let mut above = 0;
    for seed in 0..200 {
        let mut random = LegacyRandom::new(seed);
        let spot =
            water_bound_random_position(&scene, &profile, near_surface, 10, 7, None, &mut random);
        if let Some(spot) = spot {
            above += usize::from(!scene.floatable_fluid(block_of(spot)));
        }
    }
    assert!(above > 20, "{above}");
}

#[test]
fn a_water_bound_spot_stays_inside_its_home() {
    let mut scene = Scene::default();
    scene.fill((-40, 50, -20), (40, 63, 20), "minecraft:water");
    let profile = swimmer();
    let home = ((0, 56, 0), 6);
    let position = DVec3::new(0.5, 56.0, 0.5);
    let mut found = 0;
    for seed in 0..200 {
        let mut random = LegacyRandom::new(seed);
        let spot =
            water_bound_random_position(&scene, &profile, position, 10, 7, Some(home), &mut random);
        let Some(spot) = spot else { continue };
        found += 1;
        let (x, y, z) = block_of(spot);
        assert!(x * x + (y - 56) * (y - 56) + z * z < 36, "{spot:?}");
    }
    assert!(found > 100, "{found}");
    let far = DVec3::new(30.5, 56.0, 0.5);
    let mut beyond_ten = 0;
    for seed in 0..200 {
        let mut random = LegacyRandom::new(seed);
        let spot =
            water_bound_random_position(&scene, &profile, far, 10, 7, Some(home), &mut random);
        let Some(spot) = spot else { continue };
        let dx = block_of(spot).0 - 30;
        assert!(dx <= 10, "{spot:?}");
        beyond_ten += usize::from(dx < -10);
    }
    assert!(beyond_ten > 0, "{beyond_ten}");
    let mut random = LegacyRandom::new(3);
    for _ in 0..200 {
        let spot = water_bound_random_position(&scene, &profile, far, 10, 7, None, &mut random);
        assert!(spot.is_some_and(|spot| (block_of(spot).0 - 30).abs() <= 10));
    }
}
