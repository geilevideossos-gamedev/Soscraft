use crate::firework::{Explosion, Shape};
use crate::firework_rocket::{
    BLAST_FAR_SOUND, BLAST_SOUND, BLAST_VOLUME, Burst, FAR_FROM_CAMERA_SQR,
};
use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::rng::LegacyRandom;

pub const LARGE_BLAST_SOUND: &str = "minecraft:entity.firework_rocket.large_blast";
pub const LARGE_BLAST_FAR_SOUND: &str = "minecraft:entity.firework_rocket.large_blast_far";
pub const TWINKLE_SOUND: &str = "minecraft:entity.firework_rocket.twinkle";
pub const TWINKLE_FAR_SOUND: &str = "minecraft:entity.firework_rocket.twinkle_far";
pub const BURST_SPARKS: usize = 70;
pub const SPARK_FRAMES: i32 = 8;
pub const FLASH_LIFETIME: i32 = 4;
pub const TWINKLE_DELAY: i32 = 15;
pub const MAX_SPARKS: usize = 16384;
const CROWDED_SPARKS: usize = 12288;
const BLACK: i32 = 1973019;
const GRAVITY: f64 = 0.1;
const FRICTION: f64 = 0.91;
const FULL_BRIGHT: f32 = 15.0;
const STAR_COORDS: [[f64; 2]; 6] = [
    [0.0, 1.0],
    [0.3455, 0.309],
    [0.9511, 0.309],
    [0.3795918367346939, -0.12653061224489795],
    [0.6122448979591837, -0.8040816326530612],
    [0.0, -0.35918367346938773],
];
const CREEPER_COORDS: [[f64; 2]; 12] = [
    [0.0, 0.2],
    [0.2, 0.2],
    [0.2, 0.6],
    [0.6, 0.6],
    [0.6, 0.2],
    [0.2, 0.2],
    [0.2, 0.0],
    [0.4, 0.0],
    [0.4, -0.6],
    [0.2, -0.6],
    [0.2, -0.4],
    [0.0, -0.4],
];

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
    fade: Option<[f32; 3]>,
    trail: bool,
    twinkle: bool,
}

impl Spark {
    fn shown(&self) -> bool {
        !self.twinkle || self.age < self.lifetime / 3 || ((self.age + self.lifetime) / 3) % 2 == 0
    }

    fn leaves_trail(&self) -> bool {
        self.trail && self.age < self.lifetime / 2 && (self.age + self.lifetime) % 2 == 0
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Flash {
    position: DVec3,
    age: i32,
    color: [f32; 3],
}

#[derive(Clone, Debug, PartialEq)]
struct Starter {
    position: DVec3,
    velocity: DVec3,
    explosions: Vec<Explosion>,
    life: i32,
    lifetime: i32,
    twinkle: bool,
}

impl Starter {
    fn new(burst: &Burst) -> Self {
        Self {
            position: burst.position,
            velocity: burst.velocity,
            explosions: burst.explosions.clone(),
            life: 0,
            lifetime: starter_lifetime(&burst.explosions),
            twinkle: burst.explosions.iter().any(|explosion| explosion.twinkle),
        }
    }
}

struct Look {
    colors: Vec<i32>,
    fade_colors: Vec<i32>,
    trail: bool,
    twinkle: bool,
}

impl Look {
    fn of(explosion: &Explosion) -> Self {
        let colors = if explosion.colors.is_empty() {
            vec![BLACK]
        } else {
            explosion.colors.clone()
        };
        Self {
            colors,
            fade_colors: explosion.fade_colors.clone(),
            trail: explosion.trail,
            twinkle: explosion.twinkle,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FireworkSound {
    pub event: &'static str,
    pub at: DVec3,
    pub volume: f32,
    pub pitch: f32,
}

pub fn starter_lifetime(explosions: &[Explosion]) -> i32 {
    let lifetime = explosions.len() as i32 * 2 - 1;
    if explosions.iter().any(|explosion| explosion.twinkle) {
        lifetime + TWINKLE_DELAY
    } else {
        lifetime
    }
}

pub fn blast_sound(explosions: &[Explosion], far: bool) -> &'static str {
    let large = explosions.len() >= 3
        || explosions
            .iter()
            .any(|explosion| explosion.shape == Shape::LargeBall);
    match (large, far) {
        (true, true) => LARGE_BLAST_FAR_SOUND,
        (true, false) => LARGE_BLAST_SOUND,
        (false, true) => BLAST_FAR_SOUND,
        (false, false) => BLAST_SOUND,
    }
}

pub fn twinkle_sound(far: bool) -> &'static str {
    if far {
        TWINKLE_FAR_SOUND
    } else {
        TWINKLE_SOUND
    }
}

fn new_spark(
    random: &mut LegacyRandom,
    position: DVec3,
    velocity: DVec3,
    color: [f32; 3],
) -> Spark {
    let _base_lifetime = random.next_float();
    let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0 * 0.75;
    let lifetime = 48 + random.next_int(12) as i32;
    Spark {
        position,
        previous: position,
        velocity,
        age: 0,
        lifetime,
        color,
        alpha: 0.99,
        quad_size,
        stopped: false,
        fade: None,
        trail: false,
        twinkle: false,
    }
}

fn add_spark(sparks: &mut Vec<Spark>, random: &mut LegacyRandom, spark: Spark) {
    let count = sparks.len();
    if count >= MAX_SPARKS {
        return;
    }
    if count >= CROWDED_SPARKS {
        let room = (MAX_SPARKS - count) as f32 / (MAX_SPARKS - CROWDED_SPARKS) as f32;
        if random.next_float() >= room * room {
            return;
        }
    }
    sparks.push(spark);
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
    starters: Vec<Starter>,
    queued: Vec<Starter>,
    random: LegacyRandom,
}

impl FireworkParticles {
    pub fn new(seed: u64) -> Self {
        Self {
            sparks: Vec::new(),
            flashes: Vec::new(),
            starters: Vec::new(),
            queued: Vec::new(),
            random: LegacyRandom::new(seed),
        }
    }

    fn spark(&mut self, position: DVec3, velocity: DVec3, color: [f32; 3]) {
        let spark = new_spark(&mut self.random, position, velocity, color);
        add_spark(&mut self.sparks, &mut self.random, spark);
    }

    fn star_spark(&mut self, position: DVec3, velocity: DVec3, look: &Look) {
        let mut spark = new_spark(&mut self.random, position, velocity, [1.0, 1.0, 1.0]);
        spark.trail = look.trail;
        spark.twinkle = look.twinkle;
        let color = look.colors[self.random.next_int(look.colors.len() as u32) as usize];
        spark.color = rgb(color);
        if !look.fade_colors.is_empty() {
            let fade = self.random.next_int(look.fade_colors.len() as u32) as usize;
            spark.fade = Some(rgb(look.fade_colors[fade]));
        }
        add_spark(&mut self.sparks, &mut self.random, spark);
    }

    pub fn trail(&mut self, at: DVec3, rise: f64) {
        let x = self.random.next_gaussian() * 0.05;
        let z = self.random.next_gaussian() * 0.05;
        self.spark(at, DVec3::new(x, -rise * 0.5, z), [1.0, 1.0, 1.0]);
    }

    pub fn burst(&mut self, burst: &Burst) {
        if !burst.explosions.is_empty() {
            self.queued.push(Starter::new(burst));
        }
    }

    fn explode(&mut self, at: DVec3, velocity: DVec3, explosion: &Explosion) {
        let look = Look::of(explosion);
        match explosion.shape {
            Shape::SmallBall => self.ball(at, 0.25, 2, &look),
            Shape::LargeBall => self.ball(at, 0.5, 4, &look),
            Shape::Star => self.shape(at, 0.5, &STAR_COORDS, false, &look),
            Shape::Creeper => self.shape(at, 0.5, &CREEPER_COORDS, true, &look),
            Shape::Burst => self.scatter(at, velocity, &look),
        }
        self.flashes.push(Flash {
            position: at,
            age: 0,
            color: rgb(look.colors[0]),
        });
    }

    fn ball(&mut self, at: DVec3, speed: f64, size: i32, look: &Look) {
        for y in -size..=size {
            for x in -size..=size {
                let mut z = -size;
                while z <= size {
                    let dx = f64::from(x)
                        + (self.random.next_double() - self.random.next_double()) * 0.5;
                    let dy = f64::from(y)
                        + (self.random.next_double() - self.random.next_double()) * 0.5;
                    let dz = f64::from(z)
                        + (self.random.next_double() - self.random.next_double()) * 0.5;
                    let length = (dx * dx + dy * dy + dz * dz).sqrt() / speed
                        + self.random.next_gaussian() * 0.05;
                    let velocity = DVec3::new(dx / length, dy / length, dz / length);
                    self.star_spark(at, velocity, look);
                    if y != -size && y != size && x != -size && x != size {
                        z += size * 2 - 1;
                    }
                    z += 1;
                }
            }
        }
    }

    fn shape(&mut self, at: DVec3, speed: f64, coords: &[[f64; 2]], creeper: bool, look: &Look) {
        let [first_x, first_y] = coords[0];
        self.star_spark(at, DVec3::new(first_x * speed, first_y * speed, 0.0), look);
        let start = self.random.next_float() * std::f32::consts::PI;
        let spacing = if creeper { 0.034 } else { 0.34 };
        for turn in 0..3 {
            let angle = f64::from(start) + f64::from(turn as f32 * std::f32::consts::PI) * spacing;
            let mut previous = coords[0];
            for &next in &coords[1..] {
                for step in 1..=4 {
                    let t = f64::from(step) * 0.25;
                    let across = (previous[0] + t * (next[0] - previous[0])) * speed;
                    let up = (previous[1] + t * (next[1] - previous[1])) * speed;
                    let x = across * angle.cos();
                    let z = across * angle.sin();
                    for side in [-1.0, 1.0] {
                        self.star_spark(at, DVec3::new(x * side, up, z * side), look);
                    }
                }
                previous = next;
            }
        }
    }

    fn scatter(&mut self, at: DVec3, velocity: DVec3, look: &Look) {
        let base_x = self.random.next_gaussian() * 0.05;
        let base_z = self.random.next_gaussian() * 0.05;
        for _ in 0..BURST_SPARKS {
            let x = velocity.x * 0.5 + self.random.next_gaussian() * 0.15 + base_x;
            let z = velocity.z * 0.5 + self.random.next_gaussian() * 0.15 + base_z;
            let y = velocity.y * 0.5 + self.random.next_double() * 0.5;
            self.star_spark(at, DVec3::new(x, y, z), look);
        }
    }

    fn tick_starter(
        &mut self,
        starter: &mut Starter,
        camera: DVec3,
        sounds: &mut Vec<FireworkSound>,
    ) -> bool {
        let far = camera.distance_squared(starter.position) >= FAR_FROM_CAMERA_SQR;
        if starter.life == 0 {
            sounds.push(FireworkSound {
                event: blast_sound(&starter.explosions, far),
                at: starter.position,
                volume: BLAST_VOLUME,
                pitch: 0.95 + self.random.next_float() * 0.1,
            });
        }
        let star = (starter.life / 2) as usize;
        if starter.life % 2 == 0 && star < starter.explosions.len() {
            self.explode(
                starter.position,
                starter.velocity,
                &starter.explosions[star],
            );
        }
        starter.life += 1;
        if starter.life <= starter.lifetime {
            return true;
        }
        if starter.twinkle {
            sounds.push(FireworkSound {
                event: twinkle_sound(far),
                at: starter.position,
                volume: BLAST_VOLUME,
                pitch: 0.9 + self.random.next_float() * 0.15,
            });
        }
        false
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

    pub fn tick(
        &mut self,
        camera: DVec3,
        blocked: impl Fn((i32, i32, i32)) -> bool,
    ) -> Vec<FireworkSound> {
        let random = &mut self.random;
        let mut trails = Vec::new();
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
                if let Some(fade) = spark.fade {
                    for (color, fade) in spark.color.iter_mut().zip(fade) {
                        *color += (fade - *color) * 0.2;
                    }
                }
            }
            if spark.leaves_trail() {
                let mut trail = new_spark(random, spark.position, DVec3::ZERO, spark.color);
                trail.age = trail.lifetime / 2;
                trail.twinkle = spark.twinkle;
                trails.push(trail);
            }
            true
        });
        for trail in trails {
            add_spark(&mut self.sparks, &mut self.random, trail);
        }
        self.flashes.retain_mut(|flash| {
            let age = flash.age;
            flash.age += 1;
            age < FLASH_LIFETIME
        });
        let mut sounds = Vec::new();
        let mut starters = std::mem::take(&mut self.starters);
        starters.retain_mut(|starter| self.tick_starter(starter, camera, &mut sounds));
        starters.append(&mut self.queued);
        self.starters = starters;
        sounds
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
        for spark in self.sparks.iter().filter(|spark| spark.shown()) {
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
        self.sparks.is_empty()
            && self.flashes.is_empty()
            && self.starters.is_empty()
            && self.queued.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn star(shape: Shape, colors: &[i32]) -> Explosion {
        Explosion {
            shape,
            colors: colors.to_vec(),
            fade_colors: Vec::new(),
            trail: false,
            twinkle: false,
        }
    }

    fn burst_of(explosions: Vec<Explosion>) -> Burst {
        Burst {
            position: DVec3::new(0.5, 80.0, 0.5),
            velocity: DVec3::new(0.0, 0.8, 0.0),
            explosions,
        }
    }

    fn burst() -> Burst {
        burst_of(vec![Explosion::burst(11743532)])
    }

    fn start(
        particles: &mut FireworkParticles,
        burst: &Burst,
        camera: DVec3,
    ) -> Vec<FireworkSound> {
        particles.burst(burst);
        assert!(particles.tick(camera, |_| false).is_empty());
        particles.tick(camera, |_| false)
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
    fn a_burst_waits_a_tick_then_throws_seventy_sparks_of_its_colour_with_a_flash_and_a_bang() {
        let mut particles = FireworkParticles::new(9);
        let camera = DVec3::new(0.5, 70.0, 0.5);
        particles.burst(&burst());
        assert_eq!(particles.len(), 0);
        assert!(!particles.is_empty());
        assert!(particles.tick(camera, |_| false).is_empty());
        assert_eq!((particles.len(), particles.flashes.len()), (0, 0));
        let sounds = particles.tick(camera, |_| false);
        assert_eq!(sounds.len(), 1);
        assert_eq!(
            (sounds[0].event, sounds[0].at, sounds[0].volume),
            (BLAST_SOUND, burst().position, BLAST_VOLUME)
        );
        assert!((0.95..=1.05).contains(&sounds[0].pitch));
        assert_eq!(particles.len(), BURST_SPARKS);
        assert_eq!(particles.flashes.len(), 1);
        assert!(particles.sparks.iter().all(|s| s.color == rgb(11743532)));
        assert!(
            particles
                .sparks
                .iter()
                .all(|s| (48..60).contains(&s.lifetime) && s.velocity.y >= 0.4)
        );
        let far = start(&mut particles, &burst(), DVec3::new(0.5, 40.0, 0.5));
        assert_eq!(far[0].event, BLAST_FAR_SOUND);
    }

    #[test]
    fn sparks_fall_fade_and_go_while_the_flash_lasts_four_ticks() {
        let mut particles = FireworkParticles::new(3);
        start(&mut particles, &burst(), DVec3::ZERO);
        for _ in 0..FLASH_LIFETIME {
            particles.tick(DVec3::ZERO, |_| false);
        }
        assert_eq!(particles.flashes.len(), 1);
        particles.tick(DVec3::ZERO, |_| false);
        assert!(particles.flashes.is_empty());
        for _ in 0..40 {
            particles.tick(DVec3::ZERO, |_| false);
        }
        assert!(particles.sparks.iter().all(|s| s.alpha < 0.99));
        for _ in 0..20 {
            particles.tick(DVec3::ZERO, |_| false);
        }
        assert!(particles.is_empty());
    }

    #[test]
    fn a_starter_lives_two_ticks_a_star_and_fifteen_more_with_a_twinkle() {
        let small = star(Shape::SmallBall, &[1]);
        let mut twinkling = star(Shape::Creeper, &[2]);
        twinkling.twinkle = true;
        assert_eq!(starter_lifetime(std::slice::from_ref(&small)), 1);
        assert_eq!(starter_lifetime(&vec![small.clone(); 3]), 5);
        assert_eq!(
            starter_lifetime(&[small.clone(), twinkling.clone()]),
            3 + TWINKLE_DELAY
        );
        for (camera, event) in [
            (DVec3::new(0.5, 75.0, 0.5), TWINKLE_SOUND),
            (DVec3::new(0.5, 40.0, 0.5), TWINKLE_FAR_SOUND),
        ] {
            let mut particles = FireworkParticles::new(6);
            let explosions = vec![small.clone(), twinkling.clone()];
            assert_eq!(
                start(&mut particles, &burst_of(explosions), camera).len(),
                1
            );
            let mut heard = Vec::new();
            for tick in 1..=30 {
                for sound in particles.tick(camera, |_| false) {
                    heard.push((tick, sound));
                }
            }
            assert_eq!(heard.len(), 1);
            let (tick, sound) = heard[0];
            assert_eq!((tick, sound.event, sound.volume), (18, event, BLAST_VOLUME));
            assert!((0.9..=1.05).contains(&sound.pitch));
            assert!(particles.starters.is_empty());
        }
        let mut particles = FireworkParticles::new(6);
        start(&mut particles, &burst_of(vec![small]), DVec3::ZERO);
        assert_eq!(particles.starters.len(), 1);
        assert!(particles.tick(DVec3::ZERO, |_| false).is_empty());
        assert!(particles.starters.is_empty());
    }

    #[test]
    fn the_bang_is_large_for_a_large_ball_or_three_stars_and_far_from_sixteen_blocks() {
        let small = star(Shape::SmallBall, &[1]);
        let large = star(Shape::LargeBall, &[1]);
        assert_eq!(
            blast_sound(std::slice::from_ref(&small), false),
            BLAST_SOUND
        );
        assert_eq!(blast_sound(&vec![small.clone(); 2], false), BLAST_SOUND);
        assert_eq!(
            blast_sound(std::slice::from_ref(&large), false),
            LARGE_BLAST_SOUND
        );
        assert_eq!(
            blast_sound(&vec![small.clone(); 3], false),
            LARGE_BLAST_SOUND
        );
        assert_eq!(
            blast_sound(&[small.clone(), large.clone()], false),
            LARGE_BLAST_SOUND
        );
        assert_eq!(
            blast_sound(std::slice::from_ref(&small), true),
            BLAST_FAR_SOUND
        );
        assert_eq!(blast_sound(&[large], true), LARGE_BLAST_FAR_SOUND);
        assert_eq!(twinkle_sound(false), TWINKLE_SOUND);
        assert_eq!(twinkle_sound(true), TWINKLE_FAR_SOUND);
        let mut particles = FireworkParticles::new(2);
        let edge = start(&mut particles, &burst(), DVec3::new(0.5, 64.0, 0.5));
        assert_eq!(edge[0].event, BLAST_FAR_SOUND);
        let near = start(&mut particles, &burst(), DVec3::new(0.5, 64.1, 0.5));
        assert_eq!(near[0].event, BLAST_SOUND);
        let three = start(
            &mut particles,
            &burst_of(vec![small; 3]),
            DVec3::new(0.5, 64.1, 0.5),
        );
        assert_eq!(three[0].event, LARGE_BLAST_SOUND);
    }

    #[test]
    fn each_shape_throws_the_sparks_of_its_loops() {
        for (shape, sparks) in [
            (Shape::SmallBall, 5 * 5 * 5 - 3 * 3 * 3),
            (Shape::LargeBall, 9 * 9 * 9 - 7 * 7 * 7),
            (Shape::Star, 1 + 3 * 5 * 4 * 2),
            (Shape::Creeper, 1 + 3 * 11 * 4 * 2),
            (Shape::Burst, BURST_SPARKS),
        ] {
            let mut particles = FireworkParticles::new(8);
            start(
                &mut particles,
                &burst_of(vec![star(shape, &[1])]),
                DVec3::ZERO,
            );
            assert_eq!(particles.len(), sparks, "{shape:?}");
            assert_eq!(particles.flashes.len(), 1, "{shape:?}");
        }
    }

    #[test]
    fn balls_fly_out_at_their_speed_and_shapes_start_straight_up_in_mirrored_pairs() {
        for (shape, speed) in [(Shape::SmallBall, 0.25), (Shape::LargeBall, 0.5)] {
            let mut particles = FireworkParticles::new(10);
            start(
                &mut particles,
                &burst_of(vec![star(shape, &[1])]),
                DVec3::ZERO,
            );
            assert!(
                particles
                    .sparks
                    .iter()
                    .all(|s| (0.9..1.1).contains(&(s.velocity.length() / speed))),
                "{shape:?}"
            );
            assert!(particles.sparks.iter().any(|s| s.velocity.y < -0.2 * speed));
        }
        for (shape, first) in [
            (Shape::Star, DVec3::new(0.0, 0.5, 0.0)),
            (Shape::Creeper, DVec3::new(0.0, 0.1, 0.0)),
        ] {
            let mut particles = FireworkParticles::new(11);
            start(
                &mut particles,
                &burst_of(vec![star(shape, &[1])]),
                DVec3::ZERO,
            );
            assert_eq!(particles.sparks[0].velocity, first, "{shape:?}");
            for pair in particles.sparks[1..].chunks(2) {
                let (left, right) = (pair[0].velocity, pair[1].velocity);
                assert_eq!(left, DVec3::new(-right.x, right.y, -right.z), "{shape:?}");
                assert!(right.length() < 0.51, "{shape:?}");
            }
        }
    }

    #[test]
    fn stars_burst_one_every_second_tick_each_with_a_flash_in_its_first_colour() {
        let (red, green, blue) = (0xff0000, 0x00ff00, 0x0000ff);
        let explosions = vec![
            star(Shape::Burst, &[red, green]),
            star(Shape::SmallBall, &[]),
            star(Shape::Burst, &[blue]),
        ];
        let mut particles = FireworkParticles::new(12);
        start(&mut particles, &burst_of(explosions), DVec3::ZERO);
        let mut counts = vec![particles.len()];
        for _ in 0..4 {
            assert!(particles.tick(DVec3::ZERO, |_| false).is_empty());
            counts.push(particles.len());
        }
        assert_eq!(counts, vec![70, 70, 168, 168, 238]);
        let flashes: Vec<[f32; 3]> = particles.flashes.iter().map(|f| f.color).collect();
        assert_eq!(flashes, vec![rgb(red), rgb(BLACK), rgb(blue)]);
        let colors = |range: std::ops::Range<usize>| -> Vec<[f32; 3]> {
            let mut seen: Vec<[f32; 3]> = Vec::new();
            for spark in &particles.sparks[range] {
                if !seen.contains(&spark.color) {
                    seen.push(spark.color);
                }
            }
            seen.sort_by(|a, b| a.partial_cmp(b).unwrap());
            seen
        };
        assert_eq!(colors(0..70), vec![rgb(green), rgb(red)]);
        assert_eq!(colors(70..168), vec![rgb(BLACK)]);
        assert_eq!(colors(168..238), vec![rgb(blue)]);
        particles.tick(DVec3::ZERO, |_| false);
        assert!(particles.starters.is_empty());
    }

    #[test]
    fn sparks_thin_out_when_crowded_and_stop_at_the_cap() {
        let mut random = LegacyRandom::new(1);
        let one = new_spark(&mut random, DVec3::ZERO, DVec3::ZERO, [1.0, 1.0, 1.0]);
        let mut sparks = vec![one.clone(); CROWDED_SPARKS - 1];
        add_spark(&mut sparks, &mut random, one.clone());
        assert_eq!(sparks.len(), CROWDED_SPARKS);
        for _ in 0..10_000 {
            add_spark(&mut sparks, &mut random, one.clone());
        }
        assert!(
            (CROWDED_SPARKS + 1000..MAX_SPARKS - 500).contains(&sparks.len()),
            "{}",
            sparks.len()
        );
        sparks.resize(MAX_SPARKS, one.clone());
        add_spark(&mut sparks, &mut random, one);
        assert_eq!(sparks.len(), MAX_SPARKS);
    }

    #[test]
    fn a_rocket_leaves_a_white_trail_that_stops_on_the_ground() {
        let mut particles = FireworkParticles::new(5);
        particles.trail(DVec3::new(0.5, 64.2, 0.5), 0.4);
        assert_eq!(particles.sparks[0].color, [1.0, 1.0, 1.0]);
        assert!(particles.sparks[0].velocity.y < 0.0);
        for _ in 0..10 {
            particles.tick(DVec3::ZERO, |(_, y, _)| y < 64);
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

    #[test]
    fn a_fading_spark_turns_to_its_fade_colour_by_the_end_of_its_life() {
        let (red, blue, white) = (0xff0000, 0x0000ff, 0xffffff);
        let mut fading = star(Shape::SmallBall, &[red]);
        fading.fade_colors = vec![blue, white];
        let mut particles = FireworkParticles::new(14);
        let explosions = vec![fading, star(Shape::Burst, &[red])];
        start(&mut particles, &burst_of(explosions), DVec3::ZERO);
        let fades: Vec<Option<[f32; 3]>> = particles.sparks.iter().map(|s| s.fade).collect();
        assert!(fades.contains(&Some(rgb(blue))) && fades.contains(&Some(rgb(white))));
        assert!(fades.iter().all(|fade| fade.is_some()));
        for _ in 0..24 {
            particles.tick(DVec3::ZERO, |_| false);
        }
        assert!(particles.sparks.iter().all(|s| s.color == rgb(red)));
        for _ in 24..47 {
            particles.tick(DVec3::ZERO, |_| false);
        }
        let (faded, plain) = particles.sparks.split_at(98);
        assert!(plain.len() == BURST_SPARKS && plain.iter().all(|s| s.fade.is_none()));
        assert!(plain.iter().all(|s| s.color == rgb(red)));
        for spark in faded {
            let fade = spark.fade.expect("a fade colour");
            let left = 0.8f32.powi(spark.age - spark.lifetime / 2);
            for ((color, fade), start) in spark.color.iter().zip(fade).zip(rgb(red)) {
                assert!((color - (fade + (start - fade) * left)).abs() < 1e-3);
                assert!((color - fade).abs() < 0.02, "{spark:?}");
            }
        }
    }

    #[test]
    fn a_trailing_spark_leaves_still_sparks_of_its_colour_behind_in_its_first_half() {
        let mut particles = FireworkParticles::new(15);
        let (red, blue) = (rgb(0xff0000), rgb(0x0000ff));
        let at = DVec3::new(0.5, 80.0, 0.5);
        let mut spark = new_spark(&mut particles.random, at, DVec3::new(0.3, 0.0, 0.0), red);
        spark.trail = true;
        spark.twinkle = true;
        spark.fade = Some(blue);
        let lifetime = spark.lifetime;
        particles.sparks.push(spark);
        let mut born = Vec::new();
        for tick in 1..lifetime {
            particles.tick(DVec3::ZERO, |_| false);
            let parent = particles.sparks[0].clone();
            assert_eq!(parent.age, tick);
            for trail in particles.sparks[1..]
                .iter()
                .filter(|s| s.age == s.lifetime / 2)
            {
                assert_eq!(
                    (trail.position, trail.previous, trail.velocity),
                    (parent.position, parent.position, DVec3::ZERO)
                );
                assert_eq!((trail.color, trail.alpha, trail.fade), (red, 0.99, None));
                assert_eq!((trail.trail, trail.twinkle), (false, true));
                born.push(tick);
            }
        }
        let expected: Vec<i32> = (1..lifetime / 2)
            .filter(|age| (age + lifetime) % 2 == 0)
            .collect();
        assert_eq!(born, expected);
        assert!(particles.sparks[1..].iter().all(|s| s.color == red));
        assert!(particles.sparks[0].color[2] > 0.9);
    }

    #[test]
    fn a_twinkling_spark_blinks_in_threes_after_a_third_of_its_life() {
        let mut random = LegacyRandom::new(3);
        let mut spark = new_spark(&mut random, DVec3::ZERO, DVec3::ZERO, [1.0; 3]);
        spark.lifetime = 48;
        let hidden = |twinkle: bool| -> Vec<i32> {
            let mut spark = spark.clone();
            spark.twinkle = twinkle;
            (0..48)
                .filter(|&age| {
                    spark.age = age;
                    !spark.shown()
                })
                .collect()
        };
        assert_eq!(hidden(false), Vec::<i32>::new());
        assert_eq!(
            hidden(true),
            vec![
                16, 17, 21, 22, 23, 27, 28, 29, 33, 34, 35, 39, 40, 41, 45, 46, 47
            ]
        );
    }

    fn spark_atlas() -> Atlas {
        let temp = crate::test_dir::tempdir().unwrap();
        let root = temp.path();
        std::fs::write(
            root.join("pack.mcmeta"),
            r#"{"pack":{"min_format":[97,1],"max_format":[97,1]}}"#,
        )
        .unwrap();
        let folder = root.join("assets/minecraft/textures/particle");
        std::fs::create_dir_all(&folder).unwrap();
        let names = (0..SPARK_FRAMES).map(|i| format!("spark_{i}"));
        for name in names.chain(std::iter::once("flash".to_owned())) {
            image::RgbaImage::from_pixel(8, 8, image::Rgba([255, 255, 255, 255]))
                .save(folder.join(format!("{name}.png")))
                .unwrap();
        }
        let packs = crate::pack::PackStack::open(vec![root.into()]).unwrap();
        crate::mesh::make_atlas(&packs, textures()).unwrap()
    }

    #[test]
    fn a_twinkling_spark_draws_nothing_while_it_blinks_off() {
        let atlas = spark_atlas();
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![15], vec![0]);
        let mut particles = FireworkParticles::new(16);
        let at = DVec3::new(0.5, 64.5, 0.5);
        let mut spark = new_spark(&mut particles.random, at, DVec3::ZERO, [1.0; 3]);
        spark.lifetime = 48;
        spark.twinkle = true;
        for (age, faces) in [(15, 1), (16, 0), (18, 1)] {
            spark.age = age;
            particles.sparks = vec![spark.clone()];
            let mut mesh = ChunkMesh::default();
            particles.append_mesh(&mut mesh, &atlas, Vec3::Z, 0.5, &light);
            assert_eq!(mesh.faces, faces, "{age}");
        }
    }
}
