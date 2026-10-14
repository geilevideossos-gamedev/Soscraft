use glam::DVec3;
use minecraftoss_player::mth;
use minecraftoss_player::{Face, World};

pub const TYPE_ID: &str = "minecraft:evoker_fangs";
pub const WIDTH: f64 = 0.5;
pub const HEIGHT: f64 = 0.8;
pub const DAMAGE: f32 = 6.0;
pub const LIFE_TICKS: i32 = 22;
pub const ATTACK_TRIGGER: i32 = -8;
pub const REACH: f64 = 0.2;
pub const BITE_TICKS: f32 = 20.0;
pub const CRIT_LIFE: i32 = 13;
pub const ATTACK_SOUND: &str = "entity.evoker_fangs.attack";
pub const CLOSE_RANGE_SQR: f64 = 9.0;
pub const RING_RADIUS: f64 = 1.5;
pub const OUTER_RADIUS: f64 = 2.5;
pub const LINE_STEP: f64 = 1.25;
pub const OUTER_WARMUP: i32 = 3;

#[derive(Clone, Debug, PartialEq)]
pub struct EvokerFangs {
    pub position: DVec3,
    pub yaw: f32,
    pub warmup: i32,
    pub life_ticks: i32,
    pub sent_spike: bool,
    pub owner: Option<u64>,
    pub owner_uuid: Option<u128>,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FangsTick {
    pub bite: bool,
    pub spike: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FangSpot {
    pub x: f64,
    pub z: f64,
    pub yaw: f32,
    pub warmup: i32,
}

impl EvokerFangs {
    pub fn new(position: DVec3, yaw_radians: f32, warmup: i32, owner: Option<u64>) -> Self {
        Self {
            position,
            yaw: yaw_radians * 57.295_776,
            warmup,
            life_ticks: LIFE_TICKS,
            sent_spike: false,
            owner,
            owner_uuid: None,
            alive: true,
        }
    }

    pub fn tick(&mut self) -> FangsTick {
        let mut out = FangsTick::default();
        self.warmup -= 1;
        if self.warmup >= 0 {
            return out;
        }
        out.bite = self.warmup == ATTACK_TRIGGER;
        if !self.sent_spike {
            self.sent_spike = true;
            out.spike = true;
        }
        self.life_ticks -= 1;
        if self.life_ticks < 0 {
            self.alive = false;
        }
        out
    }

    pub fn bite_progress(&self, partial: f32) -> f32 {
        if !self.sent_spike {
            return 0.0;
        }
        let left = self.life_ticks - 1;
        if left <= 0 {
            1.0
        } else {
            (1.0 - (left as f32 - partial) / BITE_TICKS).clamp(0.0, 1.0)
        }
    }

    pub fn bite_box(&self) -> (DVec3, DVec3) {
        let half = WIDTH / 2.0 + REACH;
        let p = self.position;
        (
            DVec3::new(p.x - half, p.y, p.z - half),
            DVec3::new(p.x + half, p.y + HEIGHT, p.z + half),
        )
    }
}

fn is_air(world: &dyn World, pos: (i32, i32, i32)) -> bool {
    world.block(pos).is_none_or(|block| {
        matches!(
            block.id.as_str(),
            "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
        )
    })
}

pub fn floor_at(world: &dyn World, x: f64, z: f64, min_y: f64, max_y: f64) -> Option<f64> {
    let (bx, bz) = (x.floor() as i32, z.floor() as i32);
    let lowest = min_y.floor() as i32 - 1;
    let mut y = max_y.floor() as i32;
    loop {
        if world.sturdy_face((bx, y - 1, bz), Face::Up) {
            let mut offset = 0.0;
            if !is_air(world, (bx, y, bz)) {
                let boxes = world.collision_boxes((bx, y, bz));
                if !boxes.is_empty() {
                    offset = boxes.iter().map(|b| b[4]).fold(f64::MIN, f64::max);
                }
            }
            return Some(f64::from(y) + offset);
        }
        y -= 1;
        if y < lowest {
            return None;
        }
    }
}

pub fn spots(caster: DVec3, target: DVec3) -> Vec<FangSpot> {
    let angle = mth::atan2(target.z - caster.z, target.x - caster.x) as f32;
    let at = |turn: f32, distance: f64, warmup: i32| FangSpot {
        x: caster.x + f64::from(mth::cos(f64::from(turn))) * distance,
        z: caster.z + f64::from(mth::sin(f64::from(turn))) * distance,
        yaw: turn,
        warmup,
    };
    if caster.distance_squared(target) < CLOSE_RANGE_SQR {
        let ring = (0..5).map(|i| {
            at(
                angle + i as f32 * std::f32::consts::PI * 0.4,
                RING_RADIUS,
                0,
            )
        });
        let outer = (0..8).map(|i| {
            let turn = angle + i as f32 * std::f32::consts::PI * 2.0 / 8.0 + 1.256_637_1;
            at(turn, OUTER_RADIUS, OUTER_WARMUP)
        });
        ring.chain(outer).collect()
    } else {
        (0..16)
            .map(|i| at(angle, LINE_STEP * f64::from(i + 1), i))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_player::{Block, Pos};

    struct Field;

    impl World for Field {
        fn block(&self, pos: Pos) -> Option<Block> {
            (pos.1 < 64).then(|| Block::new("minecraft:stone"))
        }
        fn set_block(&mut self, _: Pos, _: Option<Block>) {}
    }

    #[test]
    fn fangs_bite_eight_ticks_after_their_warmup_and_go_23_ticks_after_it() {
        let mut fangs = EvokerFangs::new(DVec3::new(0.5, 64.0, 0.5), 0.0, 2, Some(1));
        let mut ticks = Vec::new();
        for tick in 1..=40 {
            let step = fangs.tick();
            ticks.push((tick, step, fangs.alive));
            if !fangs.alive {
                break;
            }
        }
        let spike = ticks.iter().find(|(_, s, _)| s.spike).unwrap().0;
        let bite = ticks.iter().find(|(_, s, _)| s.bite).unwrap().0;
        let gone = ticks.iter().find(|(_, _, alive)| !alive).unwrap().0;
        assert_eq!((spike, bite, gone), (3, 10, 25));
        assert_eq!(ticks.iter().filter(|(_, s, _)| s.bite).count(), 1);
    }

    #[test]
    fn the_jaws_rise_over_twenty_ticks_once_the_spike_is_sent() {
        let mut fangs = EvokerFangs::new(DVec3::ZERO, 0.0, 0, None);
        assert_eq!(fangs.bite_progress(0.5), 0.0);
        fangs.tick();
        assert!((fangs.bite_progress(0.0) - 0.0).abs() < 1e-6);
        assert!((fangs.bite_progress(0.5) - 0.025).abs() < 1e-6);
        for _ in 0..20 {
            fangs.tick();
        }
        assert_eq!(fangs.bite_progress(0.0), 1.0);
    }

    #[test]
    fn far_off_a_line_of_sixteen_runs_at_the_target_close_in_two_rings() {
        let far = spots(DVec3::new(0.5, 64.0, 0.5), DVec3::new(10.5, 64.0, 0.5));
        assert_eq!(far.len(), 16);
        assert!((far[0].x - 1.75).abs() < 1e-3 && (far[15].x - 20.5).abs() < 1e-3);
        assert_eq!(
            far.iter().map(|s| s.warmup).collect::<Vec<_>>(),
            (0..16).collect::<Vec<_>>()
        );
        let close = spots(DVec3::new(0.5, 64.0, 0.5), DVec3::new(2.5, 64.0, 0.5));
        assert_eq!(close.len(), 13);
        assert!(close[..5].iter().all(|s| s.warmup == 0));
        assert!(close[5..].iter().all(|s| s.warmup == OUTER_WARMUP));
        for (spot, radius) in close
            .iter()
            .zip([RING_RADIUS; 5].into_iter().chain([OUTER_RADIUS; 8]))
        {
            let distance = (spot.x - 0.5).hypot(spot.z - 0.5);
            assert!((distance - radius).abs() < 1e-3, "{distance}");
        }
    }

    #[test]
    fn a_fang_stands_on_the_ground_found_below_its_top() {
        assert_eq!(floor_at(&Field, 0.5, 0.5, 64.0, 65.0), Some(64.0));
        assert_eq!(floor_at(&Field, 0.5, 0.5, 66.0, 70.0), None);
    }
}
