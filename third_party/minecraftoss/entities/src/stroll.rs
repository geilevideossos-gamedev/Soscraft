//! 26.3 random stroll and panic position choice. Source:
//! `RandomStrollGoal`, `WaterAvoidingRandomStrollGoal`, `LandRandomPos`,
//! `DefaultRandomPos`, `RandomPos` and `GoalUtils` in the pinned 26.3 common
//! JAR, with `Animal.getWalkTargetValue`. Mobs here have no home, so no
//! position is restricted.
use crate::walk_path::{path_type_static, WalkProfile, WalkTarget};
use glam::DVec3;
use minecraftoss_player::{rng::LegacyRandom, World};

type BlockPos = (i32, i32, i32);

/// `getWalkTargetValue`: for animals grass below is worth 10, elsewhere
/// the light-dependent cost; monsters take the negated cost (darkness).
fn walk_target_value(world: &dyn World, profile: &WalkProfile, (x, y, z): BlockPos) -> f64 {
    match profile.walk_target {
        WalkTarget::Animal => {
            let grass = world.block((x, y - 1, z)).is_some_and(|b| b.id == "minecraft:grass_block");
            f64::from(if grass { 10.0 } else { world.light_path_cost((x, y, z)) })
        }
        WalkTarget::Monster => f64::from(-world.light_path_cost((x, y, z))),
        WalkTarget::Zero => 0.0,
        WalkTarget::Strider { in_lava } => {
            if world.fluid_is_lava((x, y, z)) {
                10.0
            } else if in_lava {
                f64::NEG_INFINITY
            } else {
                0.0
            }
        }
        WalkTarget::Hoglin { repellent } => {
            if repellent.is_some_and(|(rx, ry, rz)| {
                let (dx, dy, dz) = (f64::from(x - rx), f64::from(y - ry), f64::from(z - rz));
                dx * dx + dy * dy + dz * dz < 64.0
            }) {
                -1.0
            } else if world.block((x, y - 1, z)).is_some_and(|b| b.id == "minecraft:crimson_nylium") {
                10.0
            } else {
                0.0
            }
        }
        WalkTarget::MonsterOver(blocks) => {
            if world.block((x, y - 1, z)).is_some_and(|b| blocks.contains(&b.id.as_str())) {
                10.0
            } else {
                f64::from(-world.light_path_cost((x, y, z)))
            }
        }
    }
}

/// `RandomPos.generateRandomDirection`.
fn random_direction(random: &mut LegacyRandom, horizontal: u32, vertical: u32) -> BlockPos {
    let x = random.next_int(2 * horizontal + 1) as i32 - horizontal as i32;
    let y = random.next_int(2 * vertical + 1) as i32 - vertical as i32;
    let z = random.next_int(2 * horizontal + 1) as i32 - horizontal as i32;
    (x, y, z)
}

/// `RandomPos.generateRandomPosTowardDirection` for a mob with no home.
fn toward(position: DVec3, (dx, dy, dz): BlockPos) -> BlockPos {
    (
        (f64::from(dx) + position.x).floor() as i32,
        (f64::from(dy) + position.y).floor() as i32,
        (f64::from(dz) + position.z).floor() as i32,
    )
}

/// `GoalUtils.isOutsideLimits`, `isNotStable` (`PathNavigation.isStableDestination`:
/// a solid-render block below).
fn unstable_or_outside(world: &dyn World, profile: &WalkProfile, (x, y, z): BlockPos) -> bool {
    let stable = match profile.flying {
        Some(flight) => flight.is_stable_destination(world, (x, y, z)),
        None => world.solid_render((x, y - 1, z)) || profile.can_stand_on_fluid && world.fluid_is_lava((x, y, z)),
    };
    y < world.min_y() || y > world.max_y() || !stable
}

/// `GoalUtils.hasMalus`.
fn has_malus(world: &dyn World, profile: &WalkProfile, pos: BlockPos) -> bool {
    profile.malus(path_type_static(world, pos)) != 0.0
}

/// `RandomPos.generateRandomPos`: the best of ten candidates by walk
/// target value, the first on ties.
fn best_of(world: &dyn World, profile: &WalkProfile, candidate: impl FnMut() -> Option<BlockPos>) -> Option<DVec3> {
    best_by(|pos| walk_target_value(world, profile, pos), candidate)
}

/// `RandomPos.generateRandomPos` by a weight of its own.
fn best_by(mut weight: impl FnMut(BlockPos) -> f64, mut candidate: impl FnMut() -> Option<BlockPos>) -> Option<DVec3> {
    let mut best = None;
    let mut best_weight = f64::NEG_INFINITY;
    for _ in 0..10 {
        if let Some(pos) = candidate() {
            let weight = weight(pos);
            if weight > best_weight {
                best_weight = weight;
                best = Some(pos);
            }
        }
    }
    best.map(|(x, y, z)| DVec3::new(f64::from(x) + 0.5, f64::from(y), f64::from(z) + 0.5))
}

/// One `LandRandomPos.getPos` candidate: a stable spot moved up out of
/// solid blocks, kept unless in water or costly to stand in.
fn land_candidate(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, random: &mut LegacyRandom) -> Option<BlockPos> {
    let direction = random_direction(random, horizontal, vertical);
    let pos = toward(position, direction);
    if unstable_or_outside(world, profile, pos) {
        return None;
    }
    // `RandomPos.moveUpOutOfSolid`.
    let (x, mut y, z) = pos;
    if world.solid((x, y, z)) {
        y += 1;
        while y <= world.max_y() && world.solid((x, y, z)) {
            y += 1;
        }
    }
    let pos = (x, y, z);
    (!world.floatable_fluid(pos) && !has_malus(world, profile, pos)).then_some(pos)
}

/// `LandRandomPos.getPos`: a stable candidate is moved up out of solid
/// blocks, then kept unless in water or costly to stand in.
pub fn land_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || land_candidate(world, profile, position, horizontal, vertical, random))
}

/// `LandRandomPos.getPos(mob, horizontal, vertical, weight)`: as
/// [`land_random_position`], the candidates weighed by `weight`.
pub fn land_random_position_by(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, random: &mut LegacyRandom, weight: impl FnMut(BlockPos) -> f64) -> Option<DVec3> {
    best_by(weight, || land_candidate(world, profile, position, horizontal, vertical, random))
}

/// `DefaultRandomPos.getPos`: a stable candidate that costs nothing to
/// stand in.
pub fn default_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || {
        let direction = random_direction(random, horizontal, vertical);
        let pos = toward(position, direction);
        (!unstable_or_outside(world, profile, pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

#[derive(Clone, Debug, Default)]
pub struct StrollState {
    pub wanted: Option<DVec3>,
}

impl StrollState {
    /// `RandomStrollGoal.canUse` with `WaterAvoidingRandomStrollGoal`'s
    /// position: land positions, from farther in water, and one time in a
    /// thousand on dry land any stable one.
    pub fn can_use(&mut self, position: DVec3, no_action_time: i32, in_water: bool, world: &dyn World, profile: &WalkProfile, random: &mut LegacyRandom) -> bool {
        self.can_use_with(position, no_action_time, in_water, world, profile, random, 0.001)
    }

    /// [`StrollState::can_use`] with the goal's `probability` of any stable
    /// spot on dry land (the enderman's is 0).
    #[allow(clippy::too_many_arguments)]
    pub fn can_use_with(&mut self, position: DVec3, no_action_time: i32, in_water: bool, world: &dyn World, profile: &WalkProfile, random: &mut LegacyRandom, probability: f32) -> bool {
        if no_action_time >= 100 || random.next_int(60) != 0 {
            return false;
        }
        self.wanted = if in_water {
            land_random_position(world, profile, position, 15, 7, random).or_else(|| default_random_position(world, profile, position, 10, 7, random))
        } else if random.next_float() >= probability {
            land_random_position(world, profile, position, 10, 7, random)
        } else {
            default_random_position(world, profile, position, 10, 7, random)
        };
        self.wanted.is_some()
    }
}

/// `RandomPos.generateRandomDirectionWithinRadians`: a direction within
/// `max_radians` of (`x_dir`, `z_dir`), out to `max` blocks (the square's
/// corners cut off), `vertical` up or down.
fn random_direction_within(random: &mut LegacyRandom, min: f64, max: f64, vertical: u32, x_dir: f64, z_dir: f64, max_radians: f64) -> Option<BlockPos> {
    let center = crate::control::minecraft_atan2(z_dir, x_dir) - f64::from(std::f32::consts::FRAC_PI_2);
    let radians = center + f64::from(2.0 * random.next_float() - 1.0) * max_radians;
    // `Mth.lerp(sqrt(nextDouble), min, max) * Mth.SQRT_OF_TWO`.
    let distance = (min + random.next_double().sqrt() * (max - min)) * f64::from(std::f32::consts::SQRT_2);
    let xt = -distance * minecraftoss_player::jmath::sin(radians);
    let zt = distance * minecraftoss_player::jmath::cos(radians);
    if xt.abs() > max || zt.abs() > max {
        return None;
    }
    let yt = random.next_int(2 * vertical + 1) as i32 - vertical as i32;
    Some((xt.floor() as i32, yt, zt.floor() as i32))
}

/// `DefaultRandomPos.getPosTowards`: a stable, free spot within
/// `max_radians` of the way to `towards`.
#[allow(clippy::too_many_arguments)]
pub fn default_random_position_towards(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, towards: DVec3, max_radians: f32, random: &mut LegacyRandom) -> Option<DVec3> {
    let dir = towards - position;
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, f64::from(horizontal), vertical, dir.x, dir.z, f64::from(max_radians))?;
        let pos = toward(position, direction);
        (!unstable_or_outside(world, profile, pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

fn amphibious_unstable_or_outside(world: &dyn World, (x, y, z): BlockPos) -> bool {
    let air = world.block((x, y - 1, z)).is_none_or(|block| matches!(block.id.as_str(), "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"));
    y < world.min_y() || y > world.max_y() || air
}

pub fn amphibious_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || {
        let direction = random_direction(random, horizontal, vertical);
        let pos = toward(position, direction);
        (!amphibious_unstable_or_outside(world, pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

#[allow(clippy::too_many_arguments)]
pub fn amphibious_random_position_towards(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, towards: DVec3, max_radians: f32, random: &mut LegacyRandom) -> Option<DVec3> {
    let dir = towards - position;
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, f64::from(horizontal), vertical, dir.x, dir.z, f64::from(max_radians))?;
        let pos = toward(position, direction);
        (!amphibious_unstable_or_outside(world, pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

pub fn water_bound_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, home: Option<(BlockPos, i32)>, random: &mut LegacyRandom) -> Option<DVec3> {
    let restrict = home.is_some_and(|((x, y, z), radius)| {
        let reach = f64::from(radius) + f64::from(horizontal) + 1.0;
        DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5).distance_squared(position) < reach * reach
    });
    best_of(world, profile, || {
        let (dx, dy, dz) = random_direction(random, horizontal, vertical);
        let (mut x, mut z) = (f64::from(dx), f64::from(dz));
        if let Some(((home_x, _, home_z), _)) = home.filter(|_| horizontal > 1) {
            let half = f64::from(horizontal);
            x = if position.x > f64::from(home_x) { x - random.next_double() * half / 2.0 } else { x + random.next_double() * half / 2.0 };
            z = if position.z > f64::from(home_z) { z - random.next_double() * half / 2.0 } else { z + random.next_double() * half / 2.0 };
        }
        let pos = ((x + position.x).floor() as i32, (f64::from(dy) + position.y).floor() as i32, (z + position.z).floor() as i32);
        let outside = pos.1 < world.min_y() || pos.1 > world.max_y();
        let restricted = restrict && home.is_some_and(|(center, radius)| !within_home(center, radius, pos));
        (!outside && !restricted && !world.solid_render(pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

pub fn water_bound_random_position_away(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, avoid: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    let dir = position - avoid;
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, f64::from(horizontal), vertical, dir.x, dir.z, f64::from(std::f32::consts::FRAC_PI_2))?;
        let pos = toward(position, direction);
        let outside = pos.1 < world.min_y() || pos.1 > world.max_y();
        (!outside && !world.solid_render(pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

fn within_home((x, y, z): BlockPos, radius: i32, pos: BlockPos) -> bool {
    let d = DVec3::new(f64::from(x - pos.0), f64::from(y - pos.1), f64::from(z - pos.2));
    d.length_squared() < f64::from(radius * radius)
}

/// `DefaultRandomPos.getPosAway`: a stable, free spot within a quarter turn
/// of straight away from `avoid` (`mob.position().subtract(avoidPos)`).
pub fn default_random_position_away(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, avoid: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    let dir = position - avoid;
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, f64::from(horizontal), vertical, dir.x, dir.z, f64::from(std::f32::consts::FRAC_PI_2))?;
        let pos = toward(position, direction);
        (!unstable_or_outside(world, profile, pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

/// `LandRandomPos.getPosAway(mob, 0, horizontal, vertical, avoid)`: a land
/// spot within a quarter turn of the way away from `avoid`.
pub fn land_random_position_away(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, avoid: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    let mut dir = position - avoid;
    if dir.length() == 0.0 {
        dir = DVec3::new(random.next_double() - 0.5, 0.0, random.next_double() - 0.5);
    }
    land_random_position_in_direction(world, profile, position, horizontal, vertical, dir, random)
}

/// `LandRandomPos.getPosTowards`: a land spot within a quarter turn of the
/// way to `towards`.
pub fn land_random_position_towards(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, towards: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    land_random_position_in_direction(world, profile, position, horizontal, vertical, towards - position, random)
}

/// `LandRandomPos.getPosInDirection` with no minimum distance.
fn land_random_position_in_direction(world: &dyn World, profile: &WalkProfile, position: DVec3, horizontal: u32, vertical: u32, dir: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, f64::from(horizontal), vertical, dir.x, dir.z, f64::from(std::f32::consts::FRAC_PI_2))?;
        let pos = toward(position, direction);
        if unstable_or_outside(world, profile, pos) {
            return None;
        }
        let (x, mut y, z) = pos;
        if world.solid((x, y, z)) {
            y += 1;
            while y <= world.max_y() && world.solid((x, y, z)) {
                y += 1;
            }
        }
        let pos = (x, y, z);
        (!world.floatable_fluid(pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

fn random_direction_lowered(random: &mut LegacyRandom, max: f64, vertical: u32, lowered: i32, x_dir: f64, z_dir: f64, max_radians: f64) -> Option<BlockPos> {
    let (x, y, z) = random_direction_within(random, 0.0, max, vertical, x_dir, z_dir, max_radians)?;
    Some((x, y + lowered, z))
}

fn move_up_out_of_solid(world: &dyn World, (x, mut y, z): BlockPos) -> BlockPos {
    if world.solid((x, y, z)) {
        y += 1;
        while y <= world.max_y() && world.solid((x, y, z)) {
            y += 1;
        }
    }
    (x, y, z)
}

fn move_up_to_above_solid(world: &dyn World, (x, y, z): BlockPos, above: i32) -> BlockPos {
    if !world.solid((x, y, z)) {
        return (x, y, z);
    }
    let mut free = y + 1;
    while free <= world.max_y() && world.solid((x, free, z)) {
        free += 1;
    }
    let mut top = free;
    while top <= world.max_y() && top - free < above {
        if world.solid((x, top + 1, z)) {
            return (x, top, z);
        }
        top += 1;
    }
    (x, top, z)
}

pub fn hover_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, view: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || {
        let direction = random_direction_within(random, 0.0, 8.0, 7, view.x, view.z, f64::from(std::f32::consts::FRAC_PI_2))?;
        let (x, y, z) = toward(position, direction);
        if y < world.min_y() || y > world.max_y() || !world.solid_render((x, y, z)) {
            return None;
        }
        let above = random.next_int(3) as i32 + 1;
        let pos = move_up_to_above_solid(world, (x, y, z), above);
        (!world.floatable_fluid(pos) && !has_malus(world, profile, pos)).then_some(pos)
    })
}

pub fn air_and_water_random_position(world: &dyn World, profile: &WalkProfile, position: DVec3, view: DVec3, random: &mut LegacyRandom) -> Option<DVec3> {
    best_of(world, profile, || {
        let direction = random_direction_lowered(random, 8.0, 4, -2, view.x, view.z, f64::from(std::f32::consts::FRAC_PI_2))?;
        let (x, y, z) = toward(position, direction);
        if y < world.min_y() || y > world.max_y() {
            return None;
        }
        let pos = move_up_out_of_solid(world, (x, y, z));
        (!has_malus(world, profile, pos)).then_some(pos)
    })
}

#[cfg(test)]
mod tests;
