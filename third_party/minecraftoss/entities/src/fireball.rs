use crate::sight;
use glam::DVec3;
use minecraftoss_player::World;

pub const ACCELERATION_POWER: f64 = 0.1;
pub const INERTIA: f32 = 0.95;
pub const LIQUID_INERTIA: f32 = 0.8;
pub const LARGE_DAMAGE: f32 = 6.0;
pub const SMALL_DAMAGE: f32 = 5.0;
pub const SMALL_IGNITE_TICKS: i32 = 100;
pub const PICK_RADIUS: f64 = 1.0;
pub const WITHER_SKULL_SIZE: f64 = 0.3125;
pub const DANGEROUS_INERTIA: f32 = 0.73;
pub const WITHER_SKULL_DAMAGE: f32 = 8.0;
pub const OWNERLESS_SKULL_DAMAGE: f32 = 5.0;
pub const SKULL_KILL_HEAL: f32 = 5.0;
pub const SKULL_BLAST_POWER: f32 = 1.0;
pub const DANGEROUS_RESISTANCE: f32 = 0.8;
pub const ROTATION_LERP: f32 = 0.2;

pub fn skull_wither_ticks(difficulty: i32) -> u32 {
    match difficulty {
        2 => 200,
        3 => 800,
        _ => 0,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FireballKind {
    Large { explosion_power: i32 },
    Small,
    WitherSkull { dangerous: bool },
}

impl FireballKind {
    pub fn type_id(self) -> &'static str {
        match self {
            Self::Large { .. } => "minecraft:fireball",
            Self::Small => "minecraft:small_fireball",
            Self::WitherSkull { .. } => "minecraft:wither_skull",
        }
    }

    pub fn size(self) -> f64 {
        match self {
            Self::Large { .. } => 1.0,
            Self::Small => 0.3125,
            Self::WitherSkull { .. } => WITHER_SKULL_SIZE,
        }
    }

    pub fn redirectable(self) -> bool {
        matches!(self, Self::Large { .. })
    }

    pub fn inertia(self) -> f32 {
        match self {
            Self::WitherSkull { dangerous: true } => DANGEROUS_INERTIA,
            _ => INERTIA,
        }
    }

    pub fn wither_skull(self) -> bool {
        matches!(self, Self::WitherSkull { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shooter {
    Mob(u64),
    Player(u64),
}

#[derive(Clone, Copy, Debug)]
pub struct FireballTarget {
    pub id: u64,
    pub min: DVec3,
    pub max: DVec3,
    pub owner: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireballHit {
    pub location: DVec3,
    pub entity: Option<u64>,
    pub block: Option<(i32, i32, i32)>,
}

#[derive(Clone, Debug)]
pub struct Fireball {
    pub kind: FireballKind,
    pub position: DVec3,
    pub velocity: DVec3,
    pub acceleration_power: f64,
    pub owner: Option<Shooter>,
    pub tick_count: i32,
    pub left_owner: bool,
    pub in_water: bool,
    pub alive: bool,
    pub yaw: f32,
    pub pitch: f32,
}

fn lerp_turn(from: f32, to: f32) -> f32 {
    let mut from = from;
    while to - from < -180.0 {
        from -= 360.0;
    }
    while to - from >= 180.0 {
        from += 360.0;
    }
    from + ROTATION_LERP * (to - from)
}

impl Fireball {
    pub fn shoot(kind: FireballKind, owner: Shooter, position: DVec3, direction: DVec3) -> Self {
        Self {
            kind,
            position,
            velocity: normalize(direction) * ACCELERATION_POWER,
            acceleration_power: ACCELERATION_POWER,
            owner: Some(owner),
            tick_count: 0,
            left_owner: false,
            in_water: false,
            alive: true,
            yaw: 0.0,
            pitch: 0.0,
        }
    }

    pub fn rotate_towards_movement(&mut self) {
        let m = self.velocity;
        if m.length_squared() == 0.0 {
            return;
        }
        let horizontal = (m.x * m.x + m.z * m.z).sqrt();
        let yaw = (minecraftoss_player::mth::atan2(m.z, m.x) * 57.295_776_367_187_5) as f32 + 90.0;
        let pitch =
            (minecraftoss_player::mth::atan2(horizontal, m.y) * 57.295_776_367_187_5) as f32 - 90.0;
        self.pitch = lerp_turn(self.pitch, pitch);
        self.yaw = lerp_turn(self.yaw, yaw);
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        bounds_at(self.kind, self.position)
    }

    pub fn margin(&self) -> f64 {
        f64::from(0.0_f32.max(0.3_f32.min((self.tick_count - 2) as f32 / 20.0)))
    }

    pub fn deflect(&mut self, look: DVec3, by: Shooter, by_attack: bool) {
        self.velocity = look;
        self.owner = Some(by);
        if by_attack {
            self.acceleration_power = ACCELERATION_POWER;
        } else {
            self.acceleration_power *= 0.5;
        }
    }

    pub fn tick(
        &mut self,
        world: &impl World,
        targets: &[FireballTarget],
        owner_box: Option<(DVec3, DVec3)>,
    ) -> Option<FireballHit> {
        self.tick_count += 1;
        let inertia = f64::from(if self.in_water {
            LIQUID_INERTIA
        } else {
            self.kind.inertia()
        });
        self.velocity =
            (self.velocity + normalize(self.velocity) * self.acceleration_power) * inertia;
        let from = self.position;
        let movement = self.velocity;
        let mut to = from + movement;
        let block = sight::clip(world, from, to);
        if let Some((_, at)) = block {
            to = at;
        }
        let (min, max) = self.bounds();
        let (search_min, search_max) = sweep(min, max, movement);
        let (search_min, search_max) = (search_min - DVec3::ONE, search_max + DVec3::ONE);
        let margin = self.margin();
        let mut nearest = f64::MAX;
        let mut entity = None;
        for target in targets {
            if target.owner && !self.left_owner {
                continue;
            }
            if !overlaps(target.min, target.max, search_min, search_max) {
                continue;
            }
            let grow = DVec3::splat(margin);
            if let Some(at) = sight::clip_box(target.min - grow, target.max + grow, from, to) {
                let d = from.distance_squared(at);
                if d < nearest {
                    nearest = d;
                    entity = Some((target.id, at));
                }
            }
        }
        let hit = match (entity, block) {
            (Some((id, at)), _) => Some(FireballHit {
                location: at,
                entity: Some(id),
                block: None,
            }),
            (None, Some((pos, at)))
                if world.block(pos).is_some_and(|b| {
                    !matches!(
                        b.id.as_str(),
                        "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
                    )
                }) =>
            {
                Some(FireballHit {
                    location: at,
                    entity: None,
                    block: Some(pos),
                })
            }
            _ => None,
        };
        self.rotate_towards_movement();
        self.position = match (entity, block) {
            (Some((_, at)), _) | (None, Some((_, at))) => at,
            _ => from + movement,
        };
        if !self.left_owner {
            self.left_owner = match owner_box {
                Some((owner_min, owner_max)) => {
                    let (min, max) = self.bounds();
                    let (min, max) = sweep(min, max, self.velocity);
                    !overlaps(owner_min, owner_max, min - DVec3::ONE, max + DVec3::ONE)
                }
                None => true,
            };
        }
        let size = self.kind.size() as f32;
        self.in_water =
            crate::fluid::FluidFrame::sample(world, self.position, size, size).in_water();
        if self.position.y < f64::from(world.min_y() - 64) {
            self.alive = false;
        }
        if hit.is_some() && self.alive {
            return hit;
        }
        None
    }
}

pub fn bounds_at(kind: FireballKind, position: DVec3) -> (DVec3, DVec3) {
    let size = kind.size();
    let half = size / 2.0;
    (
        DVec3::new(position.x - half, position.y, position.z - half),
        DVec3::new(position.x + half, position.y + size, position.z + half),
    )
}

pub fn pick_box(kind: FireballKind, position: DVec3) -> (DVec3, DVec3) {
    let (min, max) = bounds_at(kind, position);
    (
        min - DVec3::splat(PICK_RADIUS),
        max + DVec3::splat(PICK_RADIUS),
    )
}

pub fn normalize(v: DVec3) -> DVec3 {
    let length = (v.x * v.x + v.y * v.y + v.z * v.z).sqrt();
    if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        DVec3::new(v.x / length, v.y / length, v.z / length)
    }
}

fn overlaps(a_min: DVec3, a_max: DVec3, b_min: DVec3, b_max: DVec3) -> bool {
    a_min.x < b_max.x
        && a_max.x > b_min.x
        && a_min.y < b_max.y
        && a_max.y > b_min.y
        && a_min.z < b_max.z
        && a_max.z > b_min.z
}

fn sweep(min: DVec3, max: DVec3, by: DVec3) -> (DVec3, DVec3) {
    let mut min = min;
    let mut max = max;
    for axis in 0..3 {
        if by[axis] < 0.0 {
            min[axis] += by[axis];
        } else if by[axis] > 0.0 {
            max[axis] += by[axis];
        }
    }
    (min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Air;
    impl World for Air {
        fn block(&self, _: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            None
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    struct Wall;
    impl World for Wall {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.2 >= 10).then(|| minecraftoss_player::Block::new("minecraft:netherrack"))
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    fn large() -> FireballKind {
        FireballKind::Large { explosion_power: 1 }
    }

    #[test]
    fn a_fireball_speeds_up_towards_two_blocks_a_tick() {
        let mut fireball = Fireball::shoot(
            large(),
            Shooter::Mob(1),
            DVec3::new(0.5, 70.0, 0.5),
            DVec3::new(0.0, 0.0, 3.0),
        );
        assert_eq!(fireball.velocity, DVec3::new(0.0, 0.0, 0.1));
        fireball.tick(&Air, &[], None);
        assert!(
            (fireball.velocity.z - 0.19).abs() < 1e-6,
            "{}",
            fireball.velocity.z
        );
        for _ in 0..400 {
            fireball.tick(&Air, &[], None);
        }
        assert!(
            (fireball.velocity.z - 1.9).abs() < 1e-5,
            "{}",
            fireball.velocity.z
        );
        assert!(fireball.left_owner);
    }

    #[test]
    fn a_fireball_strikes_the_first_wall_or_body_in_its_way_but_not_its_shooter_at_once() {
        let mut fireball = Fireball::shoot(
            large(),
            Shooter::Mob(1),
            DVec3::new(0.5, 70.0, 0.5),
            DVec3::Z,
        );
        let mut hit = None;
        for _ in 0..40 {
            hit = fireball.tick(&Wall, &[], None);
            if hit.is_some() {
                break;
            }
        }
        let hit = hit.expect("the wall stops it");
        assert_eq!((hit.block, hit.entity), (Some((0, 70, 10)), None));
        assert!((hit.location.z - 10.0).abs() < 1e-6);
        let ghast = FireballTarget {
            id: 1,
            min: DVec3::new(-1.5, 68.0, -1.5),
            max: DVec3::new(2.5, 72.0, 2.5),
            owner: true,
        };
        let player = FireballTarget {
            id: 2,
            min: DVec3::new(0.2, 70.0, 6.2),
            max: DVec3::new(0.8, 71.8, 6.8),
            owner: false,
        };
        let mut fireball = Fireball::shoot(
            large(),
            Shooter::Mob(1),
            DVec3::new(0.5, 70.0, 2.5),
            DVec3::Z,
        );
        let owner_box = Some((ghast.min, ghast.max));
        let mut struck = None;
        for _ in 0..40 {
            if let Some(hit) = fireball.tick(&Air, &[ghast, player], owner_box) {
                struck = hit.entity;
                break;
            }
        }
        assert_eq!(
            struck,
            Some(2),
            "it flies out of its ghast and into the player"
        );
    }

    #[test]
    fn a_deflected_fireball_flies_along_the_aim_and_belongs_to_whoever_struck_it() {
        let mut fireball = Fireball::shoot(
            large(),
            Shooter::Mob(1),
            DVec3::new(0.5, 70.0, 0.5),
            DVec3::Z,
        );
        for _ in 0..10 {
            fireball.tick(&Air, &[], None);
        }
        fireball.deflect(DVec3::new(0.0, 0.0, -1.0), Shooter::Player(7), true);
        assert_eq!(
            (
                fireball.velocity,
                fireball.owner,
                fireball.acceleration_power
            ),
            (DVec3::new(0.0, 0.0, -1.0), Some(Shooter::Player(7)), 0.1)
        );
        fireball.deflect(DVec3::X, Shooter::Player(8), false);
        assert_eq!(fireball.acceleration_power, 0.05);
        let (min, max) = pick_box(large(), DVec3::ZERO);
        assert_eq!(
            (min, max),
            (DVec3::new(-1.5, -1.0, -1.5), DVec3::new(1.5, 2.0, 1.5))
        );
    }

    #[test]
    fn a_dangerous_wither_skull_flies_slower_and_turns_its_face_along_its_flight() {
        let speed = |dangerous| {
            let mut skull = Fireball::shoot(
                FireballKind::WitherSkull { dangerous },
                Shooter::Mob(1),
                DVec3::new(0.5, 70.0, 0.5),
                DVec3::X,
            );
            for _ in 0..200 {
                skull.tick(&Air, &[], None);
            }
            (skull.velocity.x, skull.yaw, skull.pitch)
        };
        let (plain, yaw, pitch) = speed(false);
        let (blue, _, _) = speed(true);
        assert!((plain - 1.9).abs() < 1e-3, "{plain}");
        assert!((blue - 0.073 / 0.27).abs() < 1e-3, "{blue}");
        assert!(
            (yaw - 90.0).abs() < 1e-3 && pitch.abs() < 1e-3,
            "{yaw} {pitch}"
        );
        let mut skull = Fireball::shoot(
            FireballKind::WitherSkull { dangerous: false },
            Shooter::Mob(1),
            DVec3::new(0.5, 70.0, 0.5),
            DVec3::Z,
        );
        skull.tick(&Air, &[], None);
        assert!(
            (skull.yaw - 324.0).abs() < 1e-3,
            "a fifth of the way round: {}",
            skull.yaw
        );
        assert_eq!(
            (
                FireballKind::WitherSkull { dangerous: true }.type_id(),
                FireballKind::WitherSkull { dangerous: true }.size(),
                FireballKind::WitherSkull { dangerous: true }.redirectable()
            ),
            ("minecraft:wither_skull", 0.3125, false)
        );
        assert_eq!([0, 1, 2, 3].map(skull_wither_ticks), [0, 0, 200, 800]);
    }
}
