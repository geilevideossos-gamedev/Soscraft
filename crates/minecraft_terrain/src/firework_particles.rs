use crate::firework_rocket::{
    BLAST_FAR_SOUND, BLAST_SOUND, BLAST_VOLUME, Burst, FAR_FROM_CAMERA_SQR,
};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::rng::LegacyRandom;

pub const BURST_SPARKS: usize = 70;
pub const SPARK_FRAMES: i32 = 8;
pub const FLASH_LIFETIME: i32 = 4;
const GRAVITY: f64 = 0.1;
const FRICTION: f64 = 0.91;
const FULL_BRIGHT: f32 = 15.0;

#[derive(Clone, Debug, PartialEq)]
struct Spark {
    position: DVec3,
    previous: DVec3,
    velocity: DVec3,
    age: i32,
    lifetime: i32,
    color: [f32; 3],
    alpha: f32,
    quad_size: f32,
    stopped: bool,
}

#[derive(Clone, Debug, PartialEq)]
struct Flash {
    position: DVec3,
    age: i32,
    color: [f32; 3],
}

pub fn rgb(color: i32) -> [f32; 3] {
    [
        ((color >> 16) & 255) as f32 / 255.0,
        ((color >> 8) & 255) as f32 / 255.0,
        (color & 255) as f32 / 255.0,
    ]
}

pub fn spark_texture(age: i32, lifetime: i32) -> String {
    let index = (age * (SPARK_FRAMES - 1) / lifetime.max(1)).clamp(0, SPARK_FRAMES - 1);
    format!("minecraft:particle/spark_{}", SPARK_FRAMES - 1 - index)
}

pub fn textures() -> Vec<ResourceId> {
    (0..SPARK_FRAMES)
        .map(|i| format!("minecraft:particle/spark_{i}"))
        .chain(std::iter::once("minecraft:particle/flash".to_owned()))
        .filter_map(|id| ResourceId::parse(&id).ok())
        .collect()
}

pub struct FireworkParticles {
    sparks: Vec<Spark>,
    flashes: Vec<Flash>,
    random: LegacyRandom,
}

impl FireworkParticles {
    pub fn new(seed: u64) -> Self {
        Self {
            sparks: Vec::new(),
            flashes: Vec::new(),
            random: LegacyRandom::new(seed),
        }
    }

    fn spark(&mut self, position: DVec3, velocity: DVec3, color: [f32; 3]) {
        let random = &mut self.random;
        let _base_lifetime = random.next_float();
        let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0 * 0.75;
        let lifetime = 48 + random.next_int(12) as i32;
        self.sparks.push(Spark {
            position,
            previous: position,
            velocity,
            age: 0,
            lifetime,
            color,
            alpha: 0.99,
            quad_size,
            stopped: false,
        });
    }

    pub fn trail(&mut self, at: DVec3, rise: f64) {
        let x = self.random.next_gaussian() * 0.05;
        let z = self.random.next_gaussian() * 0.05;
        self.spark(at, DVec3::new(x, -rise * 0.5, z), [1.0, 1.0, 1.0]);
    }

    pub fn burst(&mut self, burst: &Burst, camera: DVec3) -> (&'static str, f32, f32) {
        let far = camera.distance_squared(burst.position) >= FAR_FROM_CAMERA_SQR;
        let pitch = 0.95 + self.random.next_float() * 0.1;
        let colors: Vec<i32> = burst
            .explosions
            .iter()
            .flat_map(|explosion| explosion.colors.iter().copied())
            .collect();
        let colors = if colors.is_empty() {
            vec![1973019]
        } else {
            colors
        };
        let base_x = self.random.next_gaussian() * 0.05;
        let base_z = self.random.next_gaussian() * 0.05;
        for _ in 0..BURST_SPARKS {
            let x = burst.velocity.x * 0.5 + self.random.next_gaussian() * 0.15 + base_x;
            let z = burst.velocity.z * 0.5 + self.random.next_gaussian() * 0.15 + base_z;
            let y = burst.velocity.y * 0.5 + self.random.next_double() * 0.5;
            let color = colors[self.random.next_int(colors.len() as u32) as usize];
            self.spark(burst.position, DVec3::new(x, y, z), rgb(color));
        }
        self.flashes.push(Flash {
            position: burst.position,
            age: 0,
            color: rgb(colors[0]),
        });
        let sound = if far { BLAST_FAR_SOUND } else { BLAST_SOUND };
        (sound, BLAST_VOLUME, pitch)
    }

    pub fn poofs(&mut self) -> Vec<DVec3> {
        let mut motions = Vec::new();
        while (motions.len() as u32) < self.random.next_int(3) + 2 {
            let x = self.random.next_gaussian() * 0.05;
            let z = self.random.next_gaussian() * 0.05;
            motions.push(DVec3::new(x, 0.005, z));
        }
        motions
    }

    pub fn tick(&mut self, blocked: impl Fn((i32, i32, i32)) -> bool) {
        self.sparks.retain_mut(|spark| {
            spark.previous = spark.position;
            let age = spark.age;
            spark.age += 1;
            if age >= spark.lifetime {
                return false;
            }
            spark.velocity.y -= 0.04 * GRAVITY;
            if !spark.stopped {
                let next = spark.position + spark.velocity;
                let cell = (
                    next.x.floor() as i32,
                    next.y.floor() as i32,
                    next.z.floor() as i32,
                );
                if blocked(cell) {
                    spark.stopped = true;
                    spark.velocity = DVec3::ZERO;
                } else {
                    spark.position = next;
                }
            }
            spark.velocity *= FRICTION;
            if spark.age > spark.lifetime / 2 {
                let fading = (spark.age - spark.lifetime / 2) as f32 / spark.lifetime as f32;
                spark.alpha = 1.0 - fading;
            }
            true
        });
        self.flashes.retain_mut(|flash| {
            let age = flash.age;
            flash.age += 1;
            age < FLASH_LIFETIME
        });
    }

    pub fn append_mesh(
        &self,
        mesh: &mut ChunkMesh,
        atlas: &Atlas,
        forward: Vec3,
        partial: f32,
        light: &SkyLight,
    ) {
        let partial = partial.clamp(0.0, 1.0);
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let quad = |mesh: &mut ChunkMesh,
                    id: &ResourceId,
                    center: Vec3,
                    size: f32,
                    color: [f32; 4],
                    lit: (f32, f32)| {
            let [u0, v0, u1, v1] = atlas.region(id);
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in [
                (Vec2::new(-1.0, -1.0), [u0, v1]),
                (Vec2::new(-1.0, 1.0), [u0, v0]),
                (Vec2::new(1.0, 1.0), [u1, v0]),
                (Vec2::new(1.0, -1.0), [u1, v1]),
            ] {
                let point = center + (right * corner.x + up * corner.y) * size;
                mesh.vertices.push(Vertex {
                    position: point.to_array(),
                    uv,
                    color,
                    sky_light: lit.0,
                    block_light: lit.1,
                });
            }
            mesh.indices.extend_from_slice(&[
                start,
                start + 2,
                start + 1,
                start,
                start + 3,
                start + 2,
            ]);
            mesh.faces += 1;
        };
        for spark in &self.sparks {
            let Ok(id) = ResourceId::parse(&spark_texture(spark.age, spark.lifetime)) else {
                continue;
            };
            if !atlas.contains(&id) {
                continue;
            }
            let center = spark
                .previous
                .lerp(spark.position, f64::from(partial))
                .as_vec3();
            let [r, g, b] = spark.color;
            let color = [r, g, b, spark.alpha];
            quad(
                mesh,
                &id,
                center,
                spark.quad_size,
                color,
                (FULL_BRIGHT, FULL_BRIGHT),
            );
        }
        let Ok(flash) = ResourceId::parse("minecraft:particle/flash") else {
            return;
        };
        if !atlas.contains(&flash) {
            return;
        }
        for overlay in &self.flashes {
            let time = (overlay.age as f32 + partial - 1.0) * 0.25;
            let size = 7.1 * (time * std::f32::consts::PI).sin();
            let alpha = (0.6 - time * 0.5).clamp(0.0, 1.0);
            let at = overlay.position;
            let cell = (
                at.x.floor() as i32,
                at.y.floor() as i32,
                at.z.floor() as i32,
            );
            let lit = (light.get(cell) as f32, light.get_block(cell) as f32);
            let [r, g, b] = overlay.color;
            quad(mesh, &flash, at.as_vec3(), size, [r, g, b, alpha], lit);
        }
    }

    pub fn len(&self) -> usize {
        self.sparks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sparks.is_empty() && self.flashes.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn burst() -> Burst {
        Burst {
            position: DVec3::new(0.5, 80.0, 0.5),
            velocity: DVec3::new(0.0, 0.8, 0.0),
            explosions: vec![crate::firework::Explosion::burst(11743532)],
        }
    }

    #[test]
    fn a_starless_burst_puffs_two_to_four_times_and_rises_slowly() {
        let mut particles = FireworkParticles::new(4);
        let mut counts = std::collections::BTreeSet::new();
        for _ in 0..200 {
            let poofs = particles.poofs();
            counts.insert(poofs.len());
            assert!(
                poofs
                    .iter()
                    .all(|m| m.y == 0.005 && m.x.abs() < 0.5 && m.z.abs() < 0.5)
            );
        }
        assert_eq!(counts.into_iter().collect::<Vec<_>>(), vec![2, 3, 4]);
        assert!(particles.is_empty());
    }

    #[test]
    fn a_burst_throws_seventy_sparks_of_its_colour_with_a_flash_and_a_bang() {
        let mut particles = FireworkParticles::new(9);
        let (sound, volume, pitch) = particles.burst(&burst(), DVec3::new(0.5, 70.0, 0.5));
        assert_eq!((sound, volume), (BLAST_SOUND, BLAST_VOLUME));
        assert!((0.95..=1.05).contains(&pitch));
        assert_eq!(particles.len(), BURST_SPARKS);
        assert_eq!(particles.flashes.len(), 1);
        assert!(particles.sparks.iter().all(|s| s.color == rgb(11743532)));
        assert!(
            particles
                .sparks
                .iter()
                .all(|s| (48..60).contains(&s.lifetime) && s.velocity.y >= 0.4)
        );
        let (far, ..) = particles.burst(&burst(), DVec3::new(0.5, 40.0, 0.5));
        assert_eq!(far, BLAST_FAR_SOUND);
    }

    #[test]
    fn sparks_fall_fade_and_go_while_the_flash_lasts_four_ticks() {
        let mut particles = FireworkParticles::new(3);
        particles.burst(&burst(), DVec3::ZERO);
        for _ in 0..FLASH_LIFETIME {
            particles.tick(|_| false);
        }
        assert_eq!(particles.flashes.len(), 1);
        particles.tick(|_| false);
        assert!(particles.flashes.is_empty());
        for _ in 0..40 {
            particles.tick(|_| false);
        }
        assert!(particles.sparks.iter().all(|s| s.alpha < 0.99));
        for _ in 0..20 {
            particles.tick(|_| false);
        }
        assert!(particles.is_empty());
    }

    #[test]
    fn a_rocket_leaves_a_white_trail_that_stops_on_the_ground() {
        let mut particles = FireworkParticles::new(5);
        particles.trail(DVec3::new(0.5, 64.2, 0.5), 0.4);
        assert_eq!(particles.sparks[0].color, [1.0, 1.0, 1.0]);
        assert!(particles.sparks[0].velocity.y < 0.0);
        for _ in 0..10 {
            particles.tick(|(_, y, _)| y < 64);
        }
        assert!(particles.sparks[0].position.y >= 64.0);
        assert!(particles.sparks[0].stopped);
    }

    #[test]
    fn spark_sprites_run_from_the_largest_to_the_smallest() {
        assert_eq!(spark_texture(0, 56), "minecraft:particle/spark_7");
        assert_eq!(spark_texture(55, 56), "minecraft:particle/spark_1");
        assert_eq!(spark_texture(56, 56), "minecraft:particle/spark_0");
        assert_eq!(textures().len(), 9);
    }
}
