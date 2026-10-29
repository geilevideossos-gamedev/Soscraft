//! Where a villager's summoned iron golem appears (pinned 26.3
//! `SpawnUtil.trySpawnMob(IRON_GOLEM, MOB_SUMMONED, level, villager, 10, 8,
//! 6, Strategy.LEGACY_IRON_GOLEM, false)`, `IronGolem.checkSpawnObstruction`
//! and `NaturalSpawner.isValidEmptySpawnBlock`): up to ten tries, each a
//! column eight blocks about (two draws from the level's random) searched
//! down from six above for a solid block with air or liquid over it; a
//! column that has one makes a golem, drawing its facing and
//! `Mob.finalizeSpawn`'s follow-range bonus and left-handed roll from the
//! level's random, and the golem stays there if it stands on a full top
//! face with room for its height and no living thing in the way.
use glam::DVec3;
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_player::{Block, Pos, World};

/// Tries, reach across and up or down (`spawnAttempts`, `spawnRangeXZ`,
/// `spawnRangeY`).
const ATTEMPTS: u32 = 10;
const RANGE_XZ: i32 = 8;
const RANGE_Y: i32 = 6;

fn is_air(block: Option<&Block>) -> bool {
    block.is_none_or(|b| matches!(b.id.as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"))
}

/// `BlockState.liquid()`: water, lava and bubble columns.
fn is_liquid(block: Option<&Block>) -> bool {
    block.is_some_and(|b| matches!(b.id.as_str(), "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column"))
}

/// `Strategy.LEGACY_IRON_GOLEM`: not a see-through or special block (glass
/// and panes, leaves, ice, cobwebs, cactus, TNT, glowstone, beacons, sea
/// lanterns, conduits), solid (or powder snow), with air or liquid above.
fn legacy_golem_ground(world: &dyn World, pos: Pos, block: Option<&Block>, above: Option<&Block>) -> bool {
    if let Some(b) = block {
        let id = b.id.as_str();
        let excluded = matches!(
            id,
            "minecraft:cobweb"
                | "minecraft:cactus"
                | "minecraft:glass_pane"
                | "minecraft:conduit"
                | "minecraft:ice"
                | "minecraft:tnt"
                | "minecraft:glowstone"
                | "minecraft:beacon"
                | "minecraft:sea_lantern"
                | "minecraft:frosted_ice"
                | "minecraft:tinted_glass"
                | "minecraft:glass"
        ) || id.ends_with("_stained_glass_pane")
            || id.ends_with("_stained_glass")
            || id.ends_with("_leaves");
        if excluded {
            return false;
        }
    }
    (is_air(above) || is_liquid(above)) && (world.solid(pos) || block.is_some_and(|b| b.id == "minecraft:powder_snow"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strategy {
    LegacyIronGolem,
    OnTopOfCollider,
}

impl Strategy {
    fn can_spawn_on(self, world: &dyn World, pos: Pos, block: Option<&Block>, above: Option<&Block>) -> bool {
        match self {
            Self::LegacyIronGolem => legacy_golem_ground(world, pos, block, above),
            Self::OnTopOfCollider => world.collision_boxes((pos.0, pos.1 + 1, pos.2)).is_empty() && top_face_full(&world.collision_boxes(pos)),
        }
    }
}

pub trait SpawnSite {
    fn world(&self) -> &dyn World;
    fn between(&mut self, low: i32, high: i32) -> i32;
}

/// `moveToPossibleSpawnPosition`: down from `pos` (six above the start)
/// through twelve more blocks for ground; the spot above it.
fn possible_spawn_position(world: &dyn World, pos: Pos, range_y: i32, strategy: Strategy) -> Option<Pos> {
    let (x, mut y, z) = pos;
    let mut above = world.block((x, y, z));
    for _ in -range_y..=range_y {
        y -= 1;
        let current = world.block((x, y, z));
        if strategy.can_spawn_on(world, (x, y, z), current.as_ref(), above.as_ref()) {
            return Some((x, y + 1, z));
        }
        above = current;
    }
    None
}

pub fn try_spawn_mob<S: SpawnSite, T>(site: &mut S, start: Pos, attempts: u32, xz: i32, y: i32, strategy: Strategy, mut create: impl FnMut(&mut S, Pos) -> Option<T>) -> Option<T> {
    for _ in 0..attempts {
        let dx = site.between(-xz, xz);
        let dz = site.between(-xz, xz);
        let Some(pos) = possible_spawn_position(site.world(), (start.0 + dx, start.1 + y, start.2 + dz), y, strategy) else { continue };
        if let Some(spawned) = create(site, pos) {
            return Some(spawned);
        }
    }
    None
}

struct GolemSite<'a> {
    world: &'a dyn World,
    random: &'a mut LegacyRandom,
}

impl SpawnSite for GolemSite<'_> {
    fn world(&self) -> &dyn World {
        self.world
    }

    fn between(&mut self, low: i32, high: i32) -> i32 {
        self.random.next_int((high - low + 1) as u32) as i32 + low
    }
}

/// Whether a block's collision boxes fill its whole cube.
fn full_cube(boxes: &[[f64; 6]]) -> bool {
    boxes.len() == 1 && boxes[0] == [0.0, 0.0, 0.0, 1.0, 1.0, 1.0]
}

/// `Block.isFaceFull(collisionShape, UP)`: the boxes reaching the top cover
/// it (checked on a sixteenth grid).
fn top_face_full(boxes: &[[f64; 6]]) -> bool {
    let top: Vec<&[f64; 6]> = boxes.iter().filter(|b| b[4] >= 1.0).collect();
    (0..16).all(|i| {
        (0..16).all(|k| {
            let (x, z) = ((f64::from(i) + 0.5) / 16.0, (f64::from(k) + 0.5) / 16.0);
            top.iter().any(|b| b[0] <= x && x <= b[3] && b[2] <= z && z <= b[5])
        })
    })
}

/// A block holding a fluid (`getFluidState` not empty).
fn holds_fluid(block: Option<&Block>) -> bool {
    block.is_some_and(|b| {
        matches!(
            b.id.as_str(),
            "minecraft:water" | "minecraft:lava" | "minecraft:bubble_column" | "minecraft:kelp" | "minecraft:kelp_plant" | "minecraft:seagrass" | "minecraft:tall_seagrass"
        ) || b.property("waterlogged") == Some("true")
    })
}

/// `EntityType.isBlockDangerous` for the iron golem (not fire immune):
/// burning blocks (`NodeEvaluator.isBurningBlock`), wither roses, berry
/// bushes, cactus and powder snow.
fn dangerous(block: Option<&Block>) -> bool {
    block.is_some_and(|b| {
        let lit_campfire = matches!(b.id.as_str(), "minecraft:campfire" | "minecraft:soul_campfire") && b.property("lit") == Some("true");
        lit_campfire
            || matches!(
                b.id.as_str(),
                "minecraft:fire"
                    | "minecraft:soul_fire"
                    | "minecraft:lava"
                    | "minecraft:magma_block"
                    | "minecraft:lava_cauldron"
                    | "minecraft:wither_rose"
                    | "minecraft:sweet_berry_bush"
                    | "minecraft:cactus"
                    | "minecraft:powder_snow"
            )
    })
}

/// `NaturalSpawner.isValidEmptySpawnBlock` for the iron golem; the fluid
/// is not asked where the golem's feet go (`Fluids.EMPTY`).
fn valid_empty(world: &dyn World, pos: Pos, ask_fluid: bool) -> bool {
    let block = world.block(pos);
    if full_cube(&world.collision_boxes(pos)) {
        return false;
    }
    if block.as_ref().is_some_and(minecraftoss_player::redstone::is_signal_source) {
        return false;
    }
    if ask_fluid && holds_fluid(block.as_ref()) {
        return false;
    }
    // `#prevent_mob_spawning_inside`: rails.
    if block.as_ref().is_some_and(|b| b.id.ends_with("rail")) {
        return false;
    }
    !dangerous(block.as_ref())
}

/// `Mth.wrapDegrees(float)`.
fn wrap_degrees(degrees: f32) -> f32 {
    let mut r = degrees % 360.0;
    if r >= 180.0 {
        r -= 360.0;
    }
    if r < -180.0 {
        r += 360.0;
    }
    r
}

/// A summoned golem as it appears: where, its facing and its follow
/// range's spawn bonus (`RANDOM_SPAWN_BONUS_ID`, multiplying the base).
#[derive(Clone, Copy, Debug)]
pub struct SummonedGolem {
    pub at: DVec3,
    pub yaw: f32,
    pub follow_bonus: f64,
}

/// Where a summoned golem appears around `start` (the villager's block),
/// drawing from the level's random; `blocked` says whether a living thing's
/// box would meet the golem's (`isUnobstructed`).
pub fn try_spawn_golem(world: &dyn World, start: Pos, level_random: &mut LegacyRandom, blocked: impl Fn(DVec3, DVec3) -> bool) -> Option<SummonedGolem> {
    let mut site = GolemSite { world, random: level_random };
    try_spawn_mob(&mut site, start, ATTEMPTS, RANGE_XZ, RANGE_Y, Strategy::LegacyIronGolem, |site, pos| {
        let yaw = wrap_degrees(site.random.next_float() * 360.0);
        let follow_bonus = 0.114_850_000_000_000_01 * (site.random.next_double() - site.random.next_double());
        let _left_handed = site.random.next_float() < 0.05;
        let at = DVec3::new(f64::from(pos.0) + 0.5, f64::from(pos.1), f64::from(pos.2) + 0.5);
        let world = site.world;
        let below = (pos.0, pos.1 - 1, pos.2);
        if !top_face_full(&world.collision_boxes(below)) {
            return None;
        }
        if !(1..3).all(|i| valid_empty(world, (pos.0, pos.1 + i, pos.2), true)) {
            return None;
        }
        if !valid_empty(world, pos, false) {
            return None;
        }
        let half = f64::from(crate::iron_golem::WIDTH) / 2.0;
        let (min, max) = (at - DVec3::new(half, 0.0, half), at + DVec3::new(half, f64::from(crate::iron_golem::HEIGHT), half));
        if blocked(min, max) {
            return None;
        }
        Some(SummonedGolem { at, yaw, follow_bonus })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_degrees_as_mth_does() {
        assert_eq!(wrap_degrees(190.0), -170.0);
        assert_eq!(wrap_degrees(-190.0), 170.0);
        assert_eq!(wrap_degrees(359.0), -1.0);
        assert_eq!(wrap_degrees(90.0), 90.0);
    }

    #[test]
    fn top_faces_need_full_cover() {
        assert!(top_face_full(&[[0.0, 0.0, 0.0, 1.0, 1.0, 1.0]]));
        assert!(!top_face_full(&[[0.0, 0.0, 0.0, 1.0, 0.5, 1.0]]));
        assert!(top_face_full(&[[0.0, 0.5, 0.0, 0.5, 1.0, 1.0], [0.5, 0.5, 0.0, 1.0, 1.0, 1.0]]));
    }

    struct Columns;

    impl World for Columns {
        fn block(&self, (x, y, z): Pos) -> Option<Block> {
            match (x, z) {
                (1, 0) if y == 63 => Some(Block::new("minecraft:smooth_stone_slab").with("type", "bottom")),
                (1, 0) if y < 63 => None,
                (2, 0) if y == 64 => Some(Block::new("minecraft:stone")),
                (3, 0) if y == 63 => Some(Block::new("minecraft:glass")),
                _ => (y < 64).then(|| Block::new("minecraft:stone")),
            }
        }

        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    #[test]
    fn on_top_of_collider_needs_a_full_top_face() {
        let on = |x: i32| possible_spawn_position(&Columns, (x, 70, 0), 6, Strategy::OnTopOfCollider);
        assert_eq!(on(0), Some((0, 64, 0)));
        assert_eq!(on(1), None, "a slab's top is not full and there is nothing under it");
        assert_eq!(on(2), Some((2, 65, 0)), "on top of the block in the way");
        assert_eq!(on(3), Some((3, 64, 0)), "glass is a collider");
        let legacy = |x: i32| possible_spawn_position(&Columns, (x, 70, 0), 6, Strategy::LegacyIronGolem);
        assert_eq!(legacy(3), None, "the golem's rule refuses glass and finds nothing under it in reach");
        assert_eq!(legacy(0), on(0));
    }

    #[test]
    fn a_golem_still_spawns_where_it_did() {
        let mut expected = LegacyRandom::new(17);
        let mut draw = || {
            let dx = expected.next_int(17) as i32 - 8;
            let dz = expected.next_int(17) as i32 - 8;
            let yaw = wrap_degrees(expected.next_float() * 360.0);
            let bonus = 0.114_850_000_000_000_01 * (expected.next_double() - expected.next_double());
            let _ = expected.next_float();
            (dx, dz, yaw, bonus)
        };
        let (first, second) = (draw(), draw());
        let blocked_once = std::cell::Cell::new(true);
        for (blocked, (dx, dz, yaw, bonus)) in [(true, second), (false, first)] {
            blocked_once.set(blocked);
            let mut random = LegacyRandom::new(17);
            let golem = try_spawn_golem(&Columns, (20, 64, -20), &mut random, |_, _| blocked_once.replace(false)).unwrap();
            assert_eq!(golem.at, DVec3::new(f64::from(20 + dx) + 0.5, 64.0, f64::from(dz - 20) + 0.5));
            assert_eq!((golem.yaw, golem.follow_bonus), (yaw, bonus));
        }
    }
}
