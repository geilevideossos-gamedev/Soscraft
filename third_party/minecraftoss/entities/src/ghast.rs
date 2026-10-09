use crate::health::DamageState;
use crate::movement::Body;
use glam::DVec3;

pub const MAX_HEALTH: f32 = 10.0;
pub const WIDTH: f32 = 4.0;
pub const HEIGHT: f32 = 4.0;
pub const EYE_HEIGHT: f32 = 2.6;
pub const FLYING_SPEED: f64 = 0.06;
pub const FOLLOW_RANGE: f64 = 100.0;
pub const SOUND_VOLUME: f32 = 5.0;
pub const EXPERIENCE: i32 = 5;
pub const REFLECTED_FIREBALL_DAMAGE: f32 = 1000.0;

#[derive(Clone, Debug)]
pub struct Ghast {
    pub body: Body,
    pub health: f32,
    pub damage: DamageState,
    pub yaw: f32,
    pub persistence_required: bool,
    pub explosion_power: i32,
    pub charging: bool,
}

impl Ghast {
    pub fn new(position: DVec3) -> Self {
        let mut body = Body::new(position, WIDTH, HEIGHT);
        body.gravity = 0.0;
        Self {
            body,
            health: MAX_HEALTH,
            damage: DamageState::default(),
            yaw: 0.0,
            persistence_required: false,
            explosion_power: 1,
            charging: false,
        }
    }

    pub fn view_vector(&self) -> DVec3 {
        view_vector(self.yaw)
    }
}

pub fn view_vector(yaw: f32) -> DVec3 {
    let g = -yaw * (std::f64::consts::PI / 180.0) as f32;
    let h = minecraftoss_player::mth::cos(f64::from(g));
    let i = minecraftoss_player::mth::sin(f64::from(g));
    DVec3::new(f64::from(i), 0.0, f64::from(h))
}

pub fn face_toward(dx: f64, dz: f64) -> f32 {
    -(minecraftoss_player::mth::atan2(dx, dz) as f32) * 57.295776_f32
}

pub fn suitable_fly_to(position: DVec3, offsets: [f32; 3]) -> DVec3 {
    DVec3::new(
        position.x + f64::from((offsets[0] * 2.0 - 1.0) * 16.0),
        position.y + f64::from((offsets[1] * 2.0 - 1.0) * 16.0),
        position.z + f64::from((offsets[2] * 2.0 - 1.0) * 16.0),
    )
}

pub fn float_impulse(delta: DVec3) -> DVec3 {
    crate::fireball::normalize(delta) * (FLYING_SPEED * 5.0 / 3.0)
}

pub fn can_reach(
    world: &impl minecraftoss_player::World,
    position: DVec3,
    width: f32,
    height: f32,
    delta: DVec3,
) -> bool {
    let half = f64::from(width) / 2.0;
    let min = DVec3::new(position.x - half, position.y, position.z - half);
    let max = DVec3::new(
        position.x + half,
        position.y + f64::from(height),
        position.z + half,
    );
    let lo = min.min(min + delta);
    let hi = max.max(max + delta);
    let extent = (max - min) / 2.0;
    let start = (min + max) / 2.0;
    let end = start + delta;
    for x in lo.x.floor() as i32..=hi.x.floor() as i32 {
        for y in lo.y.floor() as i32..=hi.y.floor() as i32 {
            for z in lo.z.floor() as i32..=hi.z.floor() as i32 {
                let cell = DVec3::new(f64::from(x), f64::from(y), f64::from(z));
                if min.x < cell.x + 1.0
                    && max.x > cell.x
                    && min.y < cell.y + 1.0
                    && max.y > cell.y
                    && min.z < cell.z + 1.0
                    && max.z > cell.z
                {
                    continue;
                }
                if !segment_meets(start, end, cell - extent, cell + DVec3::ONE + extent) {
                    continue;
                }
                for b in world.collision_boxes((x, y, z)) {
                    let box_min = cell + DVec3::new(b[0], b[1], b[2]) - extent;
                    let box_max = cell + DVec3::new(b[3], b[4], b[5]) + extent;
                    if segment_meets(start, end, box_min, box_max) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

fn segment_meets(start: DVec3, end: DVec3, min: DVec3, max: DVec3) -> bool {
    let inside = |p: DVec3| {
        p.x > min.x && p.x < max.x && p.y > min.y && p.y < max.y && p.z > min.z && p.z < max.z
    };
    if inside(start) || inside(end) {
        return true;
    }
    let d = end - start;
    let (mut t0, mut t1) = (0.0_f64, 1.0_f64);
    for axis in 0..3 {
        if d[axis].abs() < 1e-12 {
            if start[axis] <= min[axis] || start[axis] >= max[axis] {
                return false;
            }
            continue;
        }
        let (a, b) = (
            (min[axis] - start[axis]) / d[axis],
            (max[axis] - start[axis]) / d[axis],
        );
        t0 = t0.max(a.min(b));
        t1 = t1.min(a.max(b));
        if t0 >= t1 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Pillar;
    impl minecraftoss_player::World for Pillar {
        fn block(&self, pos: minecraftoss_player::Pos) -> Option<minecraftoss_player::Block> {
            (pos.0 == 10 && pos.2 == 0)
                .then(|| minecraftoss_player::Block::new("minecraft:netherrack"))
        }
        fn set_block(
            &mut self,
            _: minecraftoss_player::Pos,
            _: Option<minecraftoss_player::Block>,
        ) {
        }
    }

    #[test]
    fn a_ghast_floats_to_a_point_within_sixteen_blocks_unless_a_block_is_in_the_way() {
        let at = DVec3::new(0.5, 80.0, 0.5);
        let target = suitable_fly_to(at, [1.0, 0.75, 0.0]);
        assert_eq!(target, DVec3::new(16.5, 88.0, -15.5));
        assert!(
            (float_impulse(DVec3::new(0.0, 3.0, 4.0)) - DVec3::new(0.0, 0.06, 0.08)).length()
                < 1e-9
        );
        assert!(
            !can_reach(&Pillar, at, WIDTH, HEIGHT, DVec3::new(16.0, 0.0, 0.0)),
            "the pillar stands in its way"
        );
        assert!(can_reach(
            &Pillar,
            at,
            WIDTH,
            HEIGHT,
            DVec3::new(-16.0, 0.0, 0.0)
        ));
        assert!(can_reach(
            &Pillar,
            at,
            WIDTH,
            HEIGHT,
            DVec3::new(0.0, 0.0, 16.0)
        ));
        assert!(
            !can_reach(
                &Pillar,
                DVec3::new(5.5, 80.0, 6.0),
                WIDTH,
                HEIGHT,
                DVec3::new(10.0, 0.0, -10.0)
            ),
            "nor diagonally past it"
        );
        assert_eq!(view_vector(0.0), DVec3::new(0.0, 0.0, 1.0));
        assert!((view_vector(90.0).x + 1.0).abs() < 1e-6);
        assert!((face_toward(1.0, 0.0) + 90.0).abs() < 1e-3);
        assert_eq!(Ghast::new(at).body.width, 4.0);
    }
}
