use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Quat, Vec2, Vec3};

pub const SPRITE: &str = "minecraft:particle/shriek";
pub const RINGS: i32 = 10;
pub const RING_DELAY: i32 = 5;
pub const TOP_Y: f64 = 0.5;
const QUAD_SIZE: f32 = 0.85;
const LIFETIME: i32 = 30;
const RISE: f64 = 0.1;
const FRICTION: f64 = 0.98;
const TILT: f32 = 1.0472;
const FULL_BRIGHT: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq)]
struct Ring {
    position: DVec3,
    previous: DVec3,
    rise: f64,
    age: i32,
    delay: i32,
}

impl Ring {
    fn tick(&mut self) -> bool {
        if self.delay > 0 {
            self.delay -= 1;
            return true;
        }
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= LIFETIME {
            return false;
        }
        self.position.y += self.rise;
        self.rise *= FRICTION;
        true
    }

    fn aged(&self, partial: f32) -> f32 {
        (self.age as f32 + partial) / LIFETIME as f32
    }

    fn size(&self, partial: f32) -> f32 {
        QUAD_SIZE * (self.aged(partial) * 0.75).clamp(0.0, 1.0)
    }

    fn alpha(&self, partial: f32) -> f32 {
        1.0 - self.aged(partial).clamp(0.0, 1.0)
    }
}

pub fn turns() -> [Quat; 2] {
    [
        Quat::from_rotation_x(-TILT),
        Quat::from_euler(glam::EulerRot::YXZ, -std::f32::consts::PI, TILT, 0.0),
    ]
}

pub fn shriek_at(pos: (i32, i32, i32)) -> DVec3 {
    DVec3::new(
        f64::from(pos.0) + 0.5,
        f64::from(pos.1) + TOP_Y,
        f64::from(pos.2) + 0.5,
    )
}

#[derive(Clone, Debug, Default)]
pub struct ShriekRings {
    rings: Vec<Ring>,
}

impl ShriekRings {
    pub fn add(&mut self, at: DVec3, delay: i32) {
        self.rings.push(Ring {
            position: at,
            previous: at,
            rise: RISE,
            age: 0,
            delay,
        });
    }

    pub fn shriek(&mut self, pos: (i32, i32, i32)) {
        let at = shriek_at(pos);
        for ring in 0..RINGS {
            self.add(at, ring * RING_DELAY);
        }
    }

    pub fn tick(&mut self) {
        self.rings.retain_mut(Ring::tick);
    }

    pub fn len(&self) -> usize {
        self.rings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rings.is_empty()
    }

    pub fn append_mesh(
        &self,
        translucent: &mut ChunkMesh,
        atlas: &Atlas,
        partial: f32,
        light: &SkyLight,
    ) {
        let Ok(sprite) = ResourceId::parse(SPRITE) else {
            return;
        };
        if !atlas.contains(&sprite) {
            return;
        }
        let [u0, v0, u1, v1] = atlas.region(&sprite);
        let partial = partial.clamp(0.0, 1.0);
        for ring in self.rings.iter().filter(|ring| ring.delay <= 0) {
            let centre = ring.previous.lerp(ring.position, f64::from(partial));
            let cell = (
                centre.x.floor() as i32,
                centre.y.floor() as i32,
                centre.z.floor() as i32,
            );
            let sky = f32::from(light.get(cell));
            let centre = centre.as_vec3();
            let size = ring.size(partial);
            let color = [1.0, 1.0, 1.0, ring.alpha(partial)];
            for turn in turns() {
                let start = translucent.vertices.len() as u32;
                for (corner, uv) in [
                    (Vec2::new(1.0, -1.0), [u1, v1]),
                    (Vec2::new(1.0, 1.0), [u1, v0]),
                    (Vec2::new(-1.0, 1.0), [u0, v0]),
                    (Vec2::new(-1.0, -1.0), [u0, v1]),
                ] {
                    let point = centre + turn * Vec3::new(corner.x, corner.y, 0.0) * size;
                    translucent.vertices.push(Vertex {
                        position: point.to_array(),
                        uv,
                        color,
                        sky_light: sky,
                        block_light: FULL_BRIGHT,
                    });
                }
                translucent.indices.extend_from_slice(&[
                    start,
                    start + 1,
                    start + 2,
                    start,
                    start + 2,
                    start + 3,
                ]);
                translucent.faces += 1;
            }
        }
    }
}

pub fn textures() -> Vec<ResourceId> {
    ResourceId::parse(SPRITE).into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shriek_sends_ten_rings_five_ticks_apart_that_rise_and_fade() {
        let mut rings = ShriekRings::default();
        rings.shriek((3, 64, -2));
        assert_eq!(rings.len(), 10);
        let at = DVec3::new(3.5, 64.5, -1.5);
        assert!(rings.rings.iter().all(|ring| ring.position == at));
        let delays: Vec<i32> = rings.rings.iter().map(|ring| ring.delay).collect();
        assert_eq!(delays, vec![0, 5, 10, 15, 20, 25, 30, 35, 40, 45]);
        rings.tick();
        let first = rings.rings[0];
        assert!((first.position.y - 64.6).abs() < 1e-9);
        assert_eq!(rings.rings[1].position, at, "the second waits");
        assert!(first.size(0.0) > 0.0 && first.size(0.0) < 0.05);
        assert!((first.alpha(0.0) - (1.0 - 1.0 / 30.0)).abs() < 1e-6);
        let mut ticks = 1;
        while !rings.is_empty() {
            rings.tick();
            ticks += 1;
        }
        assert_eq!(
            ticks,
            45 + 31,
            "the last ring starts after 45 ticks and lives 30"
        );
        let mut single = ShriekRings::default();
        single.add(at, 0);
        for _ in 0..30 {
            single.tick();
        }
        let top = single.rings[0].position.y - at.y;
        assert!(top > 2.2 && top < 2.3, "{top}");
        assert!(single.rings[0].alpha(0.0) <= 0.0);
        assert_eq!(textures().len(), 1);
    }

    #[test]
    fn the_rings_tilt_up_from_both_sides() {
        let [front, back] = turns();
        let normal = |turn: Quat| turn * Vec3::Z;
        assert!(normal(front).y > 0.8, "{}", normal(front));
        assert!(normal(back).y < -0.8 || normal(back).y > 0.8);
        assert!(normal(front).dot(normal(back)).abs() > 0.2);
    }
}
