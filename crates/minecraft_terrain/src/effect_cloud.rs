use crate::potions::{PotionContents, PotionEffect};
use glam::DVec3;
use std::collections::HashMap;

pub const RADIUS: f32 = 3.0;
pub const RADIUS_ON_USE: f32 = -0.5;
pub const DURATION: i32 = 600;
pub const WAIT_TIME: i32 = 10;
pub const REAPPLICATION_DELAY: i32 = 20;
pub const MINIMAL_RADIUS: f32 = 0.5;
pub const HEIGHT: f64 = 0.5;
pub const INSTANT_SCALE: f64 = 0.5;
const CHECK_EVERY: i32 = 5;
pub const BREATH_WAIT_TIME: i32 = 20;
pub const BREATH_COLOR: u32 = 0xCB00E5;
pub const BOTTLE_REACH: f64 = 2.0;
pub const BOTTLE_SHRINK: f32 = 0.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Breath {
    Fireball,
    Flame(u64),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Visitor {
    pub key: u64,
    pub feet: DVec3,
    pub min: DVec3,
    pub max: DVec3,
    pub affected: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CloudView {
    pub position: DVec3,
    pub radius: f32,
    pub color: u32,
    pub waiting: bool,
    pub breath: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectCloud {
    pub position: DVec3,
    pub owner: Option<u64>,
    pub radius: f32,
    pub radius_on_use: f32,
    pub radius_per_tick: f32,
    pub duration: i32,
    pub duration_on_use: i32,
    pub wait_time: i32,
    pub reapplication_delay: i32,
    pub tick_count: i32,
    pub effects: Vec<PotionEffect>,
    pub color: u32,
    pub alive: bool,
    pub breath: Option<Breath>,
    victims: HashMap<u64, i32>,
}

impl EffectCloud {
    pub fn lingering(
        position: DVec3,
        owner: Option<u64>,
        contents: &PotionContents,
        duration_scale: f32,
    ) -> Self {
        Self {
            position,
            owner,
            radius: RADIUS,
            radius_on_use: RADIUS_ON_USE,
            radius_per_tick: -RADIUS / DURATION as f32,
            duration: DURATION,
            duration_on_use: 0,
            wait_time: WAIT_TIME,
            reapplication_delay: REAPPLICATION_DELAY,
            tick_count: 0,
            effects: contents.scaled_effects(duration_scale),
            color: contents.color(),
            alive: true,
            breath: None,
            victims: HashMap::new(),
        }
    }

    pub fn dragon_breath(
        position: DVec3,
        radius: f32,
        duration: i32,
        radius_per_tick: f32,
        effect: PotionEffect,
        breath: Breath,
    ) -> Self {
        Self {
            position,
            owner: None,
            radius,
            radius_on_use: 0.0,
            radius_per_tick,
            duration,
            duration_on_use: 0,
            wait_time: BREATH_WAIT_TIME,
            reapplication_delay: REAPPLICATION_DELAY,
            tick_count: 0,
            effects: vec![effect],
            color: BREATH_COLOR,
            alive: true,
            breath: Some(breath),
            victims: HashMap::new(),
        }
    }

    pub fn waiting(&self) -> bool {
        self.tick_count < self.wait_time
    }

    pub fn bounds(&self) -> (DVec3, DVec3) {
        let r = f64::from(self.radius);
        (
            DVec3::new(self.position.x - r, self.position.y, self.position.z - r),
            DVec3::new(
                self.position.x + r,
                self.position.y + HEIGHT,
                self.position.z + r,
            ),
        )
    }

    pub fn view(&self) -> CloudView {
        CloudView {
            position: self.position,
            radius: self.radius,
            color: self.color,
            waiting: self.waiting(),
            breath: self.breath.is_some(),
        }
    }

    fn set_radius(&mut self, radius: f32) {
        self.radius = radius.clamp(0.0, 32.0);
    }

    pub fn tick(&mut self, visitors: &[Visitor]) -> Vec<u64> {
        let mut touched = Vec::new();
        if !self.alive {
            return touched;
        }
        self.tick_count += 1;
        if self.duration != -1 && self.tick_count - self.wait_time >= self.duration {
            self.alive = false;
            return touched;
        }
        if self.waiting() {
            return touched;
        }
        let mut radius = self.radius;
        if self.radius_per_tick != 0.0 {
            radius += self.radius_per_tick;
            if radius < MINIMAL_RADIUS {
                self.alive = false;
                return touched;
            }
            self.set_radius(radius);
        }
        if self.tick_count % CHECK_EVERY != 0 {
            return touched;
        }
        let now = self.tick_count;
        self.victims.retain(|_, until| now < *until);
        if self.effects.is_empty() {
            self.victims.clear();
            return touched;
        }
        let (min, max) = self.bounds();
        for visitor in visitors {
            let meets = visitor.min.x < max.x
                && visitor.max.x > min.x
                && visitor.min.y < max.y
                && visitor.max.y > min.y
                && visitor.min.z < max.z
                && visitor.max.z > min.z;
            if !meets || !visitor.affected || self.victims.contains_key(&visitor.key) {
                continue;
            }
            let (dx, dz) = (
                visitor.feet.x - self.position.x,
                visitor.feet.z - self.position.z,
            );
            if dx * dx + dz * dz > f64::from(radius * radius) {
                continue;
            }
            self.victims
                .insert(visitor.key, self.tick_count + self.reapplication_delay);
            touched.push(visitor.key);
            if self.radius_on_use != 0.0 {
                radius += self.radius_on_use;
                if radius < MINIMAL_RADIUS {
                    self.alive = false;
                    return touched;
                }
                self.set_radius(radius);
            }
            if self.duration_on_use != 0 && self.duration != -1 {
                self.duration += self.duration_on_use;
                if self.duration <= 0 {
                    self.alive = false;
                    return touched;
                }
            }
        }
        touched
    }
}

pub fn bottle_reach(feet: DVec3, height: f64) -> (DVec3, DVec3) {
    let low = feet - DVec3::new(0.3, 0.0, 0.3);
    let high = feet + DVec3::new(0.3, height, 0.3);
    (
        low - DVec3::splat(BOTTLE_REACH),
        high + DVec3::splat(BOTTLE_REACH),
    )
}

fn meets(a: (DVec3, DVec3), b: (DVec3, DVec3)) -> bool {
    a.0.x < b.1.x
        && a.1.x > b.0.x
        && a.0.y < b.1.y
        && a.1.y > b.0.y
        && a.0.z < b.1.z
        && a.1.z > b.0.z
}

pub fn breath_near(clouds: &[CloudView], reach: (DVec3, DVec3)) -> bool {
    clouds.iter().any(|cloud| {
        let r = f64::from(cloud.radius);
        let bounds = (
            cloud.position - DVec3::new(r, 0.0, r),
            cloud.position + DVec3::new(r, HEIGHT, r),
        );
        cloud.breath && meets(reach, bounds)
    })
}

pub fn bottle_breath(clouds: &mut [EffectCloud], reach: (DVec3, DVec3)) -> bool {
    let Some(cloud) = clouds
        .iter_mut()
        .find(|cloud| cloud.alive && cloud.breath.is_some() && meets(reach, cloud.bounds()))
    else {
        return false;
    };
    cloud.set_radius(cloud.radius - BOTTLE_SHRINK);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poison_cloud() -> EffectCloud {
        EffectCloud::lingering(
            DVec3::new(0.5, 64.0, 0.5),
            Some(1),
            &PotionContents::of("poison"),
            0.25,
        )
    }

    fn standing(key: u64, x: f64) -> Visitor {
        let feet = DVec3::new(x, 64.0, 0.5);
        Visitor {
            key,
            feet,
            min: feet - DVec3::new(0.3, 0.0, 0.3),
            max: feet + DVec3::new(0.3, 1.8, 0.3),
            affected: true,
        }
    }

    #[test]
    fn a_lingering_cloud_carries_a_quarter_of_the_potion() {
        let cloud = poison_cloud();
        assert_eq!(cloud.effects, vec![PotionEffect::new("poison", 225, 0)]);
        assert_eq!(cloud.color, 8889187);
        assert_eq!(cloud.radius_per_tick, -0.005);
    }

    #[test]
    fn the_cloud_is_gone_by_tick_610() {
        let mut cloud = poison_cloud();
        let mut last = 0;
        while cloud.alive {
            cloud.tick(&[]);
            last = cloud.tick_count;
        }
        assert!((500..=610).contains(&last), "{last}");
        let mut forever = poison_cloud();
        forever.radius_per_tick = 0.0;
        while forever.alive {
            forever.tick(&[]);
        }
        assert_eq!(forever.tick_count, 610);
    }

    #[test]
    fn a_victim_waits_20_ticks_and_each_use_shrinks_the_cloud() {
        let mut cloud = poison_cloud();
        let me = [standing(7, 1.5)];
        let mut hits = Vec::new();
        for _ in 0..60 {
            if !cloud.tick(&me).is_empty() {
                hits.push(cloud.tick_count);
            }
        }
        assert_eq!(hits, vec![10, 30, 50]);
        let radius = 3.0 - 0.005 * 51.0 - 0.5 * 3.0;
        assert!((cloud.radius - radius).abs() < 1e-3, "{}", cloud.radius);
    }

    #[test]
    fn only_those_inside_the_radius_and_able_are_touched() {
        let mut cloud = poison_cloud();
        let far = standing(1, 4.0);
        let mut immune = standing(2, 0.5);
        immune.affected = false;
        let mut above = standing(3, 0.5);
        above.min.y += 1.0;
        above.max.y += 1.0;
        let near = standing(4, -1.0);
        let mut touched = Vec::new();
        for _ in 0..10 {
            touched.extend(cloud.tick(&[far, immune, above, near]));
        }
        assert_eq!(touched, vec![4]);
    }

    #[test]
    fn a_glass_bottle_takes_half_a_block_from_a_dragon_s_breath_within_two_blocks() {
        let breath = |x: f64| {
            EffectCloud::dragon_breath(
                DVec3::new(x, 64.0, 0.5),
                3.0,
                600,
                0.0,
                PotionEffect::new("instant_damage", 1, 1),
                Breath::Fireball,
            )
        };
        let mut clouds = vec![poison_cloud(), breath(0.5)];
        let feet = DVec3::new(5.0, 64.0, 0.5);
        let reach = bottle_reach(feet, 1.8);
        assert!((reach.0 - DVec3::new(2.7, 62.0, -1.8)).length() < 1e-9);
        assert!((reach.1 - DVec3::new(7.3, 67.8, 2.8)).length() < 1e-9);
        assert!(breath_near(
            &clouds.iter().map(EffectCloud::view).collect::<Vec<_>>(),
            reach
        ));
        assert!(bottle_breath(&mut clouds, reach));
        assert_eq!(clouds[1].radius, 2.5);
        assert_eq!(clouds[0].radius, 3.0, "a potion's cloud is no breath");
        assert!(!clouds[0].view().breath && clouds[1].view().breath);
        let away = bottle_reach(DVec3::new(10.0, 64.0, 0.5), 1.8);
        assert!(!bottle_breath(&mut clouds, away), "out of reach");
        assert!(!breath_near(
            &clouds.iter().map(EffectCloud::view).collect::<Vec<_>>(),
            away
        ));
        let mut potion_only = vec![poison_cloud()];
        assert!(!bottle_breath(&mut potion_only, reach));
        assert!(bottle_breath(&mut clouds, reach));
        assert!(
            !bottle_breath(&mut clouds, reach),
            "a smaller cloud is out of reach"
        );
        let inside = bottle_reach(DVec3::new(0.5, 64.0, 0.5), 1.8);
        for _ in 0..10 {
            assert!(bottle_breath(&mut clouds, inside));
        }
        assert_eq!(clouds[1].radius, 0.0, "the radius stops at nothing");
        clouds[1].alive = false;
        assert!(!bottle_breath(&mut clouds, inside));
    }

    #[test]
    fn a_cloud_without_effects_touches_nobody() {
        let mut cloud =
            EffectCloud::lingering(DVec3::ZERO, None, &PotionContents::of("water"), 0.25);
        let mut touched = Vec::new();
        for _ in 0..40 {
            touched.extend(cloud.tick(&[standing(1, 0.0)]));
        }
        assert!(touched.is_empty());
        assert_eq!(cloud.color, crate::potions::DEFAULT_COLOR);
    }
}
