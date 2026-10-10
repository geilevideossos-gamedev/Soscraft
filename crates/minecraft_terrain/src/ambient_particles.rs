use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::scene::{BlockPos, Scene};
use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::{Block as PlayerBlock, collision_shape_boxes, rng::LegacyRandom};
use std::collections::HashMap;

const CAP: usize = 16384;
const GENERIC_SPRITES: i32 = 8;
const MAXIMUM_COLLISION_VELOCITY_SQUARED: f64 = 10000.0;
const FULL_BRIGHT: f32 = 15.0;

type Boxes = Vec<([f64; 3], [f64; 3])>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Smoke,
    LargeSmoke,
    Lava,
}

#[derive(Clone, Debug)]
struct Particle {
    kind: Kind,
    position: DVec3,
    previous: DVec3,
    velocity: DVec3,
    color: [f32; 3],
    alpha: f32,
    quad_size: f32,
    age: i32,
    lifetime: i32,
    gravity: f32,
    friction: f32,
    size: f64,
    speed_up: bool,
    on_ground: bool,
    stopped: bool,
}

impl Particle {
    fn base(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let lifetime = (4.0 / (random.next_float() * 0.9 + 0.1)) as i32;
        let mut jitter = || (random.next_double() * 2.0 - 1.0) * f64::from(0.4_f32);
        let mut velocity = DVec3::new(
            motion.x + jitter(),
            motion.y + jitter(),
            motion.z + jitter(),
        );
        let speed = (random.next_double() + random.next_double() + 1.0) * f64::from(0.15_f32);
        let length = velocity.length();
        if length > 0.0 {
            velocity = velocity / length * speed * f64::from(0.4_f32);
        }
        velocity.y += f64::from(0.1_f32);
        let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0;
        Self {
            kind,
            position: at,
            previous: at,
            velocity,
            color: [1.0; 3],
            alpha: 1.0,
            quad_size,
            age: 0,
            lifetime,
            gravity: 0.0,
            friction: 0.98,
            size: f64::from(0.2_f32),
            speed_up: false,
            on_ground: false,
            stopped: false,
        }
    }

    fn smoke(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let scale = if kind == Kind::LargeSmoke { 2.5 } else { 1.0 };
        let mut particle = Self::base(kind, at, DVec3::ZERO, random);
        particle.friction = 0.96;
        particle.gravity = -0.1;
        particle.speed_up = true;
        particle.velocity = particle.velocity * f64::from(0.1_f32) + motion;
        let shade = random.next_float() * 0.3;
        particle.color = [shade; 3];
        particle.quad_size *= 0.75 * scale;
        let lifetime = 8.0 / (f64::from(random.next_float()) * 0.8 + 0.2) * f64::from(scale);
        particle.lifetime = (lifetime as i32).max(1);
        particle
    }

    fn lava(at: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Lava, at, DVec3::ZERO, random);
        particle.gravity = 0.75;
        particle.friction = 0.999;
        particle.velocity *= f64::from(0.8_f32);
        particle.velocity.y = f64::from(random.next_float() * 0.4 + 0.05);
        particle.quad_size *= random.next_float() * 2.0 + 0.2;
        particle.lifetime = (16.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
        particle
    }

    fn sprite(&self) -> Option<ResourceId> {
        let name = match self.kind {
            Kind::Smoke | Kind::LargeSmoke => {
                let step = self.age * (GENERIC_SPRITES - 1) / self.lifetime.max(1);
                format!(
                    "generic_{}",
                    GENERIC_SPRITES - 1 - step.clamp(0, GENERIC_SPRITES - 1)
                )
            }
            Kind::Lava => "lava".to_owned(),
        };
        ResourceId::parse(&format!("minecraft:particle/{name}")).ok()
    }

    fn size_at(&self, partial: f32) -> f32 {
        let grown = (self.age as f32 + partial) / self.lifetime as f32;
        match self.kind {
            Kind::Smoke | Kind::LargeSmoke => self.quad_size * (grown * 32.0).clamp(0.0, 1.0),
            Kind::Lava => self.quad_size * (1.0 - grown * grown),
        }
    }

    fn tick<S: Scene + ?Sized>(&mut self, scene: &S, cache: &mut HashMap<BlockPos, Boxes>) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        self.velocity.y -= 0.04 * f64::from(self.gravity);
        self.travel(self.velocity, scene, cache);
        if self.speed_up && self.position.y == self.previous.y {
            self.velocity.x *= 1.1;
            self.velocity.z *= 1.1;
        }
        self.velocity *= f64::from(self.friction);
        if self.on_ground {
            self.velocity.x *= f64::from(0.7_f32);
            self.velocity.z *= f64::from(0.7_f32);
        }
        true
    }

    fn travel<S: Scene + ?Sized>(
        &mut self,
        wanted: DVec3,
        scene: &S,
        cache: &mut HashMap<BlockPos, Boxes>,
    ) {
        if self.stopped {
            return;
        }
        let half = self.size / 2.0;
        let p = self.position;
        let mut low = [p.x - half, p.y, p.z - half];
        let mut high = [p.x + half, p.y + self.size, p.z + half];
        let requested = wanted.to_array();
        let mut moved = requested;
        if wanted != DVec3::ZERO && wanted.length_squared() < MAXIMUM_COLLISION_VELOCITY_SQUARED {
            let boxes = boxes_around(scene, cache, low, high, requested);
            let order = if wanted.x.abs() < wanted.z.abs() {
                [1, 2, 0]
            } else {
                [1, 0, 2]
            };
            for axis in order {
                moved[axis] = clip(&boxes, &low, &high, axis, moved[axis]);
                low[axis] += moved[axis];
                high[axis] += moved[axis];
            }
        } else {
            for axis in 0..3 {
                low[axis] += moved[axis];
                high[axis] += moved[axis];
            }
        }
        if moved != [0.0; 3] {
            self.position = DVec3::new((low[0] + high[0]) / 2.0, low[1], (low[2] + high[2]) / 2.0);
        }
        let tiny = f64::from(1.0e-5_f32);
        if requested[1].abs() >= tiny && moved[1].abs() < tiny {
            self.stopped = true;
        }
        self.on_ground = requested[1] != moved[1] && requested[1] < 0.0;
        if requested[0] != moved[0] {
            self.velocity.x = 0.0;
        }
        if requested[2] != moved[2] {
            self.velocity.z = 0.0;
        }
    }
}

fn boxes_around<S: Scene + ?Sized>(
    scene: &S,
    cache: &mut HashMap<BlockPos, Boxes>,
    low: [f64; 3],
    high: [f64; 3],
    motion: [f64; 3],
) -> Boxes {
    let mut from = [0; 3];
    let mut to = [0; 3];
    for axis in 0..3 {
        from[axis] = (low[axis] + motion[axis].min(0.0)).floor() as i32 - 1;
        to[axis] = (high[axis] + motion[axis].max(0.0)).floor() as i32 + 1;
    }
    let mut out = Vec::new();
    for x in from[0]..=to[0] {
        for y in from[1]..=to[1] {
            for z in from[2]..=to[2] {
                let boxes = cache.entry((x, y, z)).or_insert_with(|| {
                    let Some(block) = scene.block((x, y, z)) else {
                        return Vec::new();
                    };
                    let shape = PlayerBlock {
                        id: block.id.key(),
                        properties: block.properties.clone(),
                    };
                    collision_shape_boxes(&shape)
                        .into_iter()
                        .map(|(mut min, mut max)| {
                            for (axis, offset) in [x, y, z].into_iter().enumerate() {
                                min[axis] += f64::from(offset);
                                max[axis] += f64::from(offset);
                            }
                            (min, max)
                        })
                        .collect()
                });
                out.extend_from_slice(boxes);
            }
        }
    }
    out
}

fn clip(boxes: &Boxes, low: &[f64; 3], high: &[f64; 3], axis: usize, mut wanted: f64) -> f64 {
    let (a, b) = ((axis + 1) % 3, (axis + 2) % 3);
    for (near, far) in boxes {
        if low[a] >= far[a] || high[a] <= near[a] || low[b] >= far[b] || high[b] <= near[b] {
            continue;
        }
        if wanted > 0.0 && high[axis] <= near[axis] {
            wanted = wanted.min(near[axis] - high[axis]);
        } else if wanted < 0.0 && low[axis] >= far[axis] {
            wanted = wanted.max(far[axis] - low[axis]);
        }
    }
    wanted
}

pub fn textures() -> Vec<ResourceId> {
    ["lava"]
        .into_iter()
        .filter_map(|name| ResourceId::parse(&format!("minecraft:particle/{name}")).ok())
        .collect()
}

pub struct AmbientParticles {
    particles: Vec<Particle>,
    random: LegacyRandom,
}

impl AmbientParticles {
    pub fn new(seed: u64) -> Self {
        Self {
            particles: Vec::new(),
            random: LegacyRandom::new(seed),
        }
    }

    pub fn len(&self) -> usize {
        self.particles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }

    pub fn clear(&mut self) {
        self.particles.clear();
    }

    pub fn add(&mut self, kind: Kind, at: DVec3, motion: DVec3) {
        let random = &mut self.random;
        let particle = match kind {
            Kind::Smoke | Kind::LargeSmoke => Particle::smoke(kind, at, motion, random),
            Kind::Lava => Particle::lava(at, random),
        };
        self.particles.push(particle);
        if self.particles.len() > CAP {
            self.particles.remove(0);
        }
    }

    pub fn tick<S: Scene + ?Sized>(&mut self, scene: &S) {
        let mut cache = HashMap::new();
        let mut trails = Vec::new();
        let random = &mut self.random;
        self.particles.retain_mut(|particle| {
            let alive = particle.tick(scene, &mut cache);
            if alive && particle.kind == Kind::Lava {
                let aged = particle.age as f32 / particle.lifetime as f32;
                if random.next_float() > aged {
                    trails.push((particle.position, particle.velocity));
                }
            }
            alive
        });
        for (at, motion) in trails {
            self.add(Kind::Smoke, at, motion);
        }
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
        for particle in &self.particles {
            let Some(sprite) = particle.sprite() else {
                continue;
            };
            if !atlas.contains(&sprite) {
                continue;
            }
            let [u0, v0, u1, v1] = atlas.region(&sprite);
            let center = particle
                .previous
                .lerp(particle.position, f64::from(partial));
            let cell = (
                center.x.floor() as i32,
                center.y.floor() as i32,
                center.z.floor() as i32,
            );
            let sky = f32::from(light.get(cell));
            let block = if particle.kind == Kind::Lava {
                FULL_BRIGHT
            } else {
                f32::from(light.get_block(cell))
            };
            let size = particle.size_at(partial);
            let center = center.as_vec3();
            let [r, g, b] = particle.color;
            let color = [r, g, b, particle.alpha];
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
                    sky_light: sky,
                    block_light: block,
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Block, ChunkPos};

    struct Blocks(HashMap<BlockPos, Block>);

    impl Scene for Blocks {
        fn block(&self, pos: BlockPos) -> Option<&Block> {
            self.0.get(&pos)
        }

        fn chunks(&self) -> Vec<ChunkPos> {
            Vec::new()
        }

        fn vertical_range(&self) -> std::ops::Range<i32> {
            -64..320
        }

        fn revision(&self) -> u64 {
            0
        }
    }

    fn open() -> Blocks {
        Blocks(HashMap::new())
    }

    #[test]
    fn smoke_is_dark_and_rises_until_it_fades() {
        let mut particles = AmbientParticles::new(1);
        for _ in 0..200 {
            particles.add(Kind::Smoke, DVec3::new(0.5, 64.0, 0.5), DVec3::ZERO);
        }
        for particle in &particles.particles {
            assert!(particle.color.iter().all(|&c| (0.0..0.3).contains(&c)));
            assert!(
                (8..=40).contains(&particle.lifetime),
                "{}",
                particle.lifetime
            );
            assert!(particle.quad_size <= 0.15 + 1e-6);
        }
        let start: f64 = particles.particles.iter().map(|p| p.position.y).sum();
        for _ in 0..6 {
            particles.tick(&open());
        }
        let later: f64 = particles.particles.iter().map(|p| p.position.y).sum();
        assert!(later > start);
        for _ in 0..40 {
            particles.tick(&open());
        }
        assert!(particles.is_empty());
    }

    #[test]
    fn large_smoke_is_two_and_a_half_times_bigger_and_longer() {
        let mut particles = AmbientParticles::new(2);
        for _ in 0..200 {
            particles.add(Kind::LargeSmoke, DVec3::ZERO, DVec3::ZERO);
        }
        assert!(
            particles
                .particles
                .iter()
                .all(|p| (20..=100).contains(&p.lifetime))
        );
        assert!(
            particles
                .particles
                .iter()
                .any(|p| p.quad_size > 0.15 && p.quad_size <= 0.375 + 1e-6)
        );
    }

    #[test]
    fn smoke_grows_in_over_its_first_thirty_second_of_life() {
        let mut particles = AmbientParticles::new(3);
        particles.add(Kind::Smoke, DVec3::ZERO, DVec3::ZERO);
        let particle = &particles.particles[0];
        assert_eq!(particle.size_at(0.0), 0.0);
        assert_eq!(particle.size_at(1.0), particle.quad_size);
        assert_eq!(
            particle.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/generic_7")
        );
    }

    #[test]
    fn smoke_stops_under_a_ceiling() {
        let scene = Blocks(HashMap::from([((0, 65, 0), Block::new("minecraft:stone"))]));
        let mut particles = AmbientParticles::new(4);
        particles.add(Kind::LargeSmoke, DVec3::new(0.5, 64.5, 0.5), DVec3::ZERO);
        for _ in 0..20 {
            particles.tick(&scene);
        }
        let particle = &particles.particles[0];
        assert!(particle.position.y + particle.size <= 65.0 + 1e-9);
        assert!(particle.stopped);
    }

    #[test]
    fn lava_leaps_falls_glows_and_trails_smoke() {
        let mut particles = AmbientParticles::new(5);
        particles.add(Kind::Lava, DVec3::new(0.5, 11.0, 0.5), DVec3::ZERO);
        let first = particles.particles[0].clone();
        assert!(first.velocity.y >= 0.05 && first.velocity.y < 0.45);
        assert!((16..=80).contains(&first.lifetime));
        assert_eq!(
            first.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/lava")
        );
        let mut highest = first.position.y;
        for _ in 0..first.lifetime {
            particles.tick(&open());
            if let Some(lava) = particles.particles.iter().find(|p| p.kind == Kind::Lava) {
                highest = highest.max(lava.position.y);
            }
        }
        assert!(highest > first.position.y);
        assert!(particles.particles.iter().any(|p| p.kind == Kind::Smoke));
        let lava = particles
            .particles
            .iter()
            .find(|p| p.kind == Kind::Lava)
            .unwrap();
        assert!(lava.position.y < highest);
        assert!(lava.size_at(0.0) < first.quad_size);
    }

    #[test]
    fn lava_lands_on_the_floor() {
        let scene = Blocks(HashMap::from([((0, 10, 0), Block::new("minecraft:stone"))]));
        let mut particles = AmbientParticles::new(6);
        particles.add(Kind::Lava, DVec3::new(0.5, 11.0, 0.5), DVec3::ZERO);
        for _ in 0..15 {
            particles.tick(&scene);
        }
        let lava = particles
            .particles
            .iter()
            .find(|p| p.kind == Kind::Lava)
            .unwrap();
        assert!(lava.position.y >= 11.0 - 1e-9);
    }
}
