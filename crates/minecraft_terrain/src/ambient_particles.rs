use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use crate::scene::{BlockPos, HandcraftedScene, Scene};
use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::{Block as PlayerBlock, collision_shape_boxes, rng::LegacyRandom};
use minecraftoss_player::mth;
use std::collections::HashMap;

const CAP: usize = 16384;
const GENERIC_SPRITES: i32 = 8;
const MAXIMUM_COLLISION_VELOCITY_SQUARED: f64 = 10000.0;
const FULL_BRIGHT: f32 = 15.0;
const BIG_SMOKE_SPRITES: u32 = 12;
const CAMPFIRE_FADE_TICKS: i32 = 60;
const CAMPFIRE_SLOT_OFFSET: f32 = 0.3125;
const DOUSE_PUFFS: usize = 20;
const SIDE_STEPS: [(i32, i32); 4] = [(0, 1), (-1, 0), (0, -1), (1, 0)];
const EXPLOSION_SPRITES: i32 = 16;
const EMITTER_TICKS: i32 = 8;
const EMITTER_FLASHES: usize = 6;
const MAX_BLAST_PARTICLES: usize = 512;
const SMALL_BLAST_RADIUS: f32 = 2.0;
const SPLASH_SPRITES: u32 = 4;
const GLITTER_SPRITES: i32 = 8;
const SCULK_SOUL_SPRITES: i32 = 11;
const SCULK_CHARGE_SPRITES: i32 = 7;
const SCULK_CHARGE_POP_SPRITES: i32 = 4;
const RISING_DRIFT: f64 = 0.009999999776482582;
const END_ROD_FADE: [f32; 3] = [242.0 / 255.0, 222.0 / 255.0, 201.0 / 255.0];
const BREATH_RED: (f32, f32) = (0.7176471, 0.8745098);
const BREATH_BLUE: (f32, f32) = (0.8235294, 0.9764706);
const BREATH_SPRITES: i32 = 3;
const BREATH_FIRST_SPRITE: i32 = 5;
const TRIAL_DETECTION_SPRITES: i32 = 5;
const TRIAL_DETECTION_SCALE: f32 = 1.5;
const OMINOUS_SPAWNING_START: [f32; 3] = [69.0 / 255.0, 174.0 / 255.0, 254.0 / 255.0];
const OMINOUS_SPAWNING_SPREAD: f64 = 0.4;
pub const OMINOUS_SPAWNING_PERIOD: i64 = 5;
const VAULT_CONNECTION_SCALE: f32 = 1.5;
const VAULT_CONNECTION_DROP: f32 = 1.2;
const VAULT_CONNECTION_ALPHA: f32 = 0.6;
const WAITING_CLOUD_PUFFS: usize = 2;
const WAITING_CLOUD_RADIUS: f32 = 0.2;
const BURST_PARTICLES: usize = 200;
const MOUTH_POINTS: usize = 8;
const FLAME_SPEEDS: usize = 6;
const TOTEM_EMITTER_TICKS: i32 = 30;
const TOTEM_TRIES: usize = 16;

type Boxes = Vec<([f64; 3], [f64; 3])>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Smoke,
    LargeSmoke,
    Lava,
    CosySmoke,
    SignalSmoke,
    Poof,
    Explosion,
    ExplosionEmitter,
    HappyVillager,
    Rain,
    Flame,
    SmallFlame,
    SoulFireFlame,
    TrialSpawnerDetection,
    TrialSpawnerDetectionOminous,
    TrialOmen,
    OminousSpawning,
    VaultConnection,
    Snowflake,
    EndRod,
    Crit,
    DragonBreath,
    Totem,
    Heart,
    AngryVillager,
    Splash,
    SculkSoul,
    SculkCharge,
    SculkChargePop,
    Spit,
}

#[derive(Clone, Debug)]
struct Particle {
    kind: Kind,
    position: DVec3,
    previous: DVec3,
    home: DVec3,
    velocity: DVec3,
    color: [f32; 3],
    alpha: f32,
    quad_size: f32,
    sprite: u32,
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
            home: at,
            velocity,
            color: [1.0; 3],
            alpha: 1.0,
            quad_size,
            sprite: 0,
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

    fn flame(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(kind, at, DVec3::ZERO, random);
        particle.friction = 0.96;
        particle.velocity = particle.velocity * f64::from(0.01_f32) + motion;
        let mut jitter = || f64::from((random.next_float() - random.next_float()) * 0.05);
        particle.position += DVec3::new(jitter(), jitter(), jitter());
        particle.lifetime = (8.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32 + 4;
        if kind == Kind::SmallFlame {
            particle.quad_size *= 0.5;
        }
        particle
    }

    fn trial_detection(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(kind, at, DVec3::ZERO, random);
        particle.friction = 0.96;
        particle.gravity = -0.1;
        particle.speed_up = true;
        particle.velocity = particle.velocity * DVec3::new(0.0, 0.9, 0.0) + motion;
        particle.quad_size *= 0.75 * TRIAL_DETECTION_SCALE;
        let between = random.next_float() * 0.5 + 0.5;
        particle.lifetime = ((8.0 / between * TRIAL_DETECTION_SCALE) as i32).max(1);
        particle
    }

    fn spell(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let x = 0.5 - random.next_double();
        let z = 0.5 - random.next_double();
        let mut particle = Self::base(kind, at, DVec3::new(x, motion.y, z), random);
        particle.friction = 0.96;
        particle.gravity = -0.1;
        particle.speed_up = true;
        particle.velocity.y *= f64::from(0.2_f32);
        if motion.x == 0.0 && motion.z == 0.0 {
            particle.velocity.x *= f64::from(0.1_f32);
            particle.velocity.z *= f64::from(0.1_f32);
        }
        particle.quad_size *= 0.75;
        particle.lifetime = (8.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
        particle
    }

    fn ominous_spawning(at: DVec3, offset: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::OminousSpawning, at, DVec3::ZERO, random);
        particle.position = at + offset;
        particle.previous = particle.position;
        particle.velocity = offset;
        particle.quad_size = 0.1 * (random.next_float() * 0.5 + 0.2);
        particle.lifetime = (random.next_float() * 5.0) as i32 + 25;
        particle.quad_size *= random.next_float() * 2.0 + 3.0;
        particle.color = OMINOUS_SPAWNING_START;
        particle
    }

    fn vault_connection(at: DVec3, offset: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::VaultConnection, at, DVec3::ZERO, random);
        particle.position = at + offset;
        particle.previous = particle.position;
        particle.velocity = offset;
        particle.quad_size = 0.1 * (random.next_float() * 0.5 + 0.2) * VAULT_CONNECTION_SCALE;
        let blue = random.next_float() * 0.6 + 0.4;
        particle.color = [0.9 * blue, 0.9 * blue, blue];
        particle.lifetime = (random.next_float() * 10.0) as i32 + 30;
        particle.alpha = 0.0;
        particle
    }

    fn fly_to_keyhole(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        let done = self.age as f32 / self.lifetime as f32;
        let left = 1.0 - done;
        let drop = done * done * done * done * VAULT_CONNECTION_DROP;
        self.position = self.home + self.velocity * f64::from(left) - DVec3::new(0.0, f64::from(drop), 0.0);
        self.alpha = ((done - 0.25) / 0.75).clamp(0.0, 1.0) * VAULT_CONNECTION_ALPHA;
        true
    }

    fn fly_home(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        let done = self.age as f32 / self.lifetime as f32;
        let left = 1.0 - done;
        self.position = self.home + self.velocity * f64::from(left);
        for (color, start) in self.color.iter_mut().zip(OMINOUS_SPAWNING_START) {
            *color = start + (1.0 - start) * done;
        }
        true
    }

    fn rise(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        self.position += self.velocity;
        self.velocity *= f64::from(self.friction);
        true
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

    fn campfire_smoke(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let signal = kind == Kind::SignalSmoke;
        let mut particle = Self::base(kind, at, DVec3::ZERO, random);
        particle.sprite = random.next_int(BIG_SMOKE_SPRITES);
        particle.quad_size *= 3.0;
        particle.size = 0.25;
        particle.lifetime = random.next_int(50) as i32 + if signal { 280 } else { 80 };
        particle.gravity = 3.0e-6;
        let rise = f64::from(random.next_float() / 500.0);
        particle.velocity = DVec3::new(motion.x, motion.y + rise, motion.z);
        particle.alpha = if signal { 0.95 } else { 0.9 };
        particle
    }

    fn poof(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Poof, at, motion, random);
        let mut jitter = || f64::from((random.next_float() * 2.0 - 1.0) * 0.05);
        particle.velocity = DVec3::new(
            motion.x + jitter(),
            motion.y + jitter(),
            motion.z + jitter(),
        );
        let shade = random.next_float() * 0.3 + 0.7;
        particle.color = [shade; 3];
        particle.quad_size = 0.1 * (random.next_float() * random.next_float() * 6.0 + 1.0);
        particle.lifetime = (16.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32 + 2;
        particle.gravity = -0.1;
        particle.friction = 0.9;
        particle
    }

    fn spit(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::poof(at, motion, random);
        particle.kind = Kind::Spit;
        particle.gravity = 0.5;
        particle
    }

    fn huge_explosion(at: DVec3, size: f64, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Explosion, at, DVec3::ZERO, random);
        particle.velocity = DVec3::ZERO;
        particle.lifetime = 6 + random.next_int(4) as i32;
        let shade = random.next_float() * 0.6 + 0.4;
        particle.color = [shade; 3];
        particle.quad_size = 2.0 * (1.0 - size as f32 * 0.5);
        particle
    }

    fn emitter(at: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::ExplosionEmitter, at, DVec3::ZERO, random);
        particle.velocity = DVec3::ZERO;
        particle.lifetime = EMITTER_TICKS;
        particle
    }

    fn sculk_soul(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::SculkSoul, at, motion, random);
        particle.friction = 0.96;
        particle.velocity = particle.velocity * RISING_DRIFT + motion;
        let mut jitter = || f64::from((random.next_float() - random.next_float()) * 0.05);
        particle.position += DVec3::new(jitter(), jitter(), jitter());
        particle.previous = particle.position;
        particle.lifetime = (8.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32 + 4;
        particle.quad_size *= 1.5;
        particle.size = f64::from(0.2_f32 * 1.5);
        particle
    }

    fn sculk_charge(kind: Kind, at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(kind, at, motion, random);
        particle.friction = 0.96;
        particle.velocity = motion;
        if kind == Kind::SculkCharge {
            particle.quad_size *= 1.5;
            particle.size = f64::from(0.2_f32 * 1.5);
            particle.lifetime = 8 + random.next_int(12) as i32;
        } else {
            particle.lifetime = 6 + random.next_int(4) as i32;
        }
        particle
    }

    fn happy_villager(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::HappyVillager, at, motion, random);
        particle.size = f64::from(0.02_f32);
        particle.quad_size *= random.next_float() * 0.6 + 0.5;
        particle.velocity *= f64::from(0.02_f32);
        particle.lifetime = (20.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
        particle
    }

    fn heart(kind: Kind, at: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(kind, at, DVec3::ZERO, random);
        particle.speed_up = true;
        particle.friction = 0.86;
        particle.velocity *= f64::from(0.01_f32);
        particle.velocity.y += 0.1;
        particle.quad_size *= 1.5;
        particle.lifetime = 16;
        particle
    }

    fn rain(at: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Rain, at, DVec3::ZERO, random);
        particle.sprite = random.next_int(SPLASH_SPRITES);
        particle.velocity.x *= f64::from(0.3_f32);
        particle.velocity.y = f64::from(random.next_float() * 0.2 + 0.1);
        particle.velocity.z *= f64::from(0.3_f32);
        particle.size = f64::from(0.01_f32);
        particle.gravity = 0.06;
        particle.lifetime = (8.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
        particle
    }

    fn splash(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::rain(at, random);
        particle.kind = Kind::Splash;
        particle.gravity = 0.04;
        if motion.y == 0.0 && (motion.x != 0.0 || motion.z != 0.0) {
            particle.velocity = DVec3::new(motion.x, 0.1, motion.z);
        }
        particle
    }

    fn snowflake(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Snowflake, at, DVec3::ZERO, random);
        particle.gravity = 0.225;
        particle.friction = 1.0;
        let mut jitter = || f64::from((random.next_float() * 2.0 - 1.0) * 0.05);
        particle.velocity = DVec3::new(
            motion.x + jitter(),
            motion.y + jitter(),
            motion.z + jitter(),
        );
        particle.quad_size = 0.1 * (random.next_float() * random.next_float() + 1.0);
        particle.lifetime = (16.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32 + 2;
        particle
    }

    fn end_rod(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::EndRod, at, DVec3::ZERO, random);
        particle.velocity = motion;
        particle.gravity = 0.0125;
        particle.friction = 0.91;
        particle.quad_size *= 0.75;
        particle.lifetime = 60 + random.next_int(12) as i32;
        particle
    }

    fn crit(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        let mut particle = Self::base(Kind::Crit, at, motion, random);
        particle.friction = 0.7;
        particle.gravity = 0.5;
        particle.velocity = particle.velocity * f64::from(0.1_f32) + motion * 0.4;
        let shade = random.next_float() * 0.3 + 0.6;
        particle.color = [shade; 3];
        particle.quad_size *= 0.75;
        particle.lifetime = ((6.0 / (f64::from(random.next_float()) * 0.8 + 0.6)) as i32).max(1);
        particle.coast();
        particle
    }

    fn dragon_breath(at: DVec3, motion: DVec3, power: f32, random: &mut LegacyRandom) -> Self {
        random.next_float();
        let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0;
        let between = |random: &mut LegacyRandom, (low, high): (f32, f32)| random.next_float() * (high - low) + low;
        let red = between(random, BREATH_RED);
        let blue = between(random, BREATH_BLUE);
        let lifetime = (20.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
        let power = f64::from(power);
        let tenth = f64::from(0.1_f32);
        Self {
            kind: Kind::DragonBreath,
            position: at,
            previous: at,
            home: at,
            velocity: DVec3::new(motion.x * power, (motion.y - tenth) * power + tenth, motion.z * power),
            color: [red, 0.0, blue],
            alpha: 1.0,
            quad_size: quad_size * 0.75,
            sprite: 0,
            age: 0,
            lifetime,
            gravity: 0.0,
            friction: 0.96,
            size: f64::from(0.2_f32),
            speed_up: false,
            on_ground: false,
            stopped: false,
        }
    }

    fn totem(at: DVec3, motion: DVec3, random: &mut LegacyRandom) -> Self {
        random.next_float();
        let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0;
        let lifetime = 60 + random.next_int(12) as i32;
        let color = if random.next_int(4) == 0 {
            [
                0.6 + random.next_float() * 0.2,
                0.6 + random.next_float() * 0.3,
                random.next_float() * 0.2,
            ]
        } else {
            [
                0.1 + random.next_float() * 0.2,
                0.4 + random.next_float() * 0.3,
                random.next_float() * 0.2,
            ]
        };
        Self {
            kind: Kind::Totem,
            position: at,
            previous: at,
            home: at,
            velocity: motion,
            color,
            alpha: 1.0,
            quad_size: quad_size * 0.75,
            sprite: 0,
            age: 0,
            lifetime,
            gravity: 1.25,
            friction: 0.6,
            size: f64::from(0.2_f32),
            speed_up: false,
            on_ground: false,
            stopped: false,
        }
    }

    fn fade_after_half_life(&mut self) {
        let half = self.lifetime / 2;
        if self.age > half {
            self.alpha = 1.0 - (self.age - half) as f32 / self.lifetime as f32;
        }
    }

    fn breathe(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        self.position += self.velocity;
        if self.position.y == self.previous.y {
            self.velocity.x *= 1.1;
            self.velocity.z *= 1.1;
        }
        self.velocity.x *= f64::from(self.friction);
        self.velocity.z *= f64::from(self.friction);
        true
    }

    fn coast(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        self.velocity.y -= 0.04 * f64::from(self.gravity);
        self.position += self.velocity;
        self.velocity *= f64::from(self.friction);
        match self.kind {
            Kind::EndRod => {
                let half = self.lifetime / 2;
                if self.age > half {
                    self.alpha = 1.0 - (self.age - half) as f32 / self.lifetime as f32;
                    for (color, fade) in self.color.iter_mut().zip(END_ROD_FADE) {
                        *color += (fade - *color) * 0.2;
                    }
                }
            }
            Kind::Crit => {
                self.color[1] *= 0.96;
                self.color[2] *= 0.9;
            }
            _ => {}
        }
        true
    }

    fn drip<S: Scene + ?Sized>(
        &mut self,
        scene: &S,
        cache: &mut HashMap<BlockPos, Boxes>,
        random: &mut LegacyRandom,
    ) -> bool {
        self.previous = self.position;
        let left = self.lifetime;
        self.lifetime -= 1;
        if left <= 0 {
            return false;
        }
        self.velocity.y -= f64::from(self.gravity);
        self.travel(self.velocity, scene, cache);
        self.velocity *= f64::from(0.98_f32);
        if self.on_ground {
            if random.next_float() < 0.5 {
                return false;
            }
            self.velocity.x *= f64::from(0.7_f32);
            self.velocity.z *= f64::from(0.7_f32);
        }
        let cell = self.position.floor();
        let pos = (cell.x as i32, cell.y as i32, cell.z as i32);
        let inside = self.position - cell;
        let top = crate::rain::surface(scene, pos, inside.x, inside.z);
        !(top > 0.0 && self.position.y < cell.y + top)
    }

    fn linger(&mut self) -> bool {
        self.previous = self.position;
        let left = self.lifetime;
        self.lifetime -= 1;
        if left <= 0 {
            return false;
        }
        self.position += self.velocity;
        self.velocity *= 0.99;
        true
    }

    fn hold(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        prior < self.lifetime
    }

    fn glow(&self, block: f32, partial: f32) -> f32 {
        let aged = ((self.age as f32 + partial) / self.lifetime as f32).clamp(0.0, 1.0);
        (block * 16.0 + (aged * 240.0) as i32 as f32).min(240.0) / 16.0
    }

    fn translucent(&self) -> bool {
        matches!(
            self.kind,
            Kind::CosySmoke
                | Kind::SignalSmoke
                | Kind::EndRod
                | Kind::Totem
                | Kind::TrialOmen
                | Kind::VaultConnection
                | Kind::SculkSoul
                | Kind::SculkCharge
                | Kind::SculkChargePop
        )
    }

    fn sprite(&self) -> Option<ResourceId> {
        let name = match self.kind {
            Kind::Smoke | Kind::LargeSmoke | Kind::Poof | Kind::Snowflake => {
                let step = self.age * (GENERIC_SPRITES - 1) / self.lifetime.max(1);
                format!(
                    "generic_{}",
                    GENERIC_SPRITES - 1 - step.clamp(0, GENERIC_SPRITES - 1)
                )
            }
            Kind::Lava => "lava".to_owned(),
            Kind::CosySmoke | Kind::SignalSmoke => format!("big_smoke_{}", self.sprite),
            Kind::Explosion => {
                let step = self.age * (EXPLOSION_SPRITES - 1) / self.lifetime.max(1);
                format!("explosion_{}", step.clamp(0, EXPLOSION_SPRITES - 1))
            }
            Kind::ExplosionEmitter => return None,
            Kind::HappyVillager => "glint".to_owned(),
            Kind::Rain => format!("splash_{}", self.sprite),
            Kind::Flame | Kind::SmallFlame => "flame".to_owned(),
            Kind::SoulFireFlame => "soul_fire_flame".to_owned(),
            Kind::TrialSpawnerDetection | Kind::TrialSpawnerDetectionOminous => {
                let step = self.age * (TRIAL_DETECTION_SPRITES - 1) / self.lifetime.max(1);
                let sheet = if self.kind == Kind::TrialSpawnerDetection {
                    "trial_spawner_detection"
                } else {
                    "trial_spawner_detection_ominous"
                };
                format!("{sheet}_{}", step.clamp(0, TRIAL_DETECTION_SPRITES - 1))
            }
            Kind::TrialOmen => "trial_omen".to_owned(),
            Kind::OminousSpawning => "ominous_spawning".to_owned(),
            Kind::VaultConnection => "vault_connection".to_owned(),
            Kind::EndRod | Kind::Totem => {
                let step = self.age * (GLITTER_SPRITES - 1) / self.lifetime.max(1);
                format!(
                    "glitter_{}",
                    GLITTER_SPRITES - 1 - step.clamp(0, GLITTER_SPRITES - 1)
                )
            }
            Kind::Crit => "critical_hit".to_owned(),
            Kind::SculkSoul => {
                let step = self.age * (SCULK_SOUL_SPRITES - 1) / self.lifetime.max(1);
                format!("sculk_soul_{}", step.clamp(0, SCULK_SOUL_SPRITES - 1))
            }
            Kind::SculkCharge => {
                let step = self.age * (SCULK_CHARGE_SPRITES - 1) / self.lifetime.max(1);
                format!("sculk_charge_{}", step.clamp(0, SCULK_CHARGE_SPRITES - 1))
            }
            Kind::SculkChargePop => {
                let step = self.age * (SCULK_CHARGE_POP_SPRITES - 1) / self.lifetime.max(1);
                format!("sculk_charge_pop_{}", step.clamp(0, SCULK_CHARGE_POP_SPRITES - 1))
            }
            Kind::DragonBreath => {
                let step = self.age * (BREATH_SPRITES - 1) / self.lifetime.max(1);
                format!("generic_{}", BREATH_FIRST_SPRITE + step.clamp(0, BREATH_SPRITES - 1))
            }
            Kind::Heart => "heart".to_owned(),
            Kind::AngryVillager => "angry".to_owned(),
            Kind::Splash => format!("splash_{}", self.sprite),
            Kind::Spit => {
                let step = self.age * (GENERIC_SPRITES - 1) / self.lifetime.max(1);
                format!("generic_{}", GENERIC_SPRITES - 1 - step.clamp(0, GENERIC_SPRITES - 1))
            }
        };
        ResourceId::parse(&format!("minecraft:particle/{name}")).ok()
    }

    fn size_at(&self, partial: f32) -> f32 {
        let grown = (self.age as f32 + partial) / self.lifetime as f32;
        match self.kind {
            Kind::Smoke
            | Kind::LargeSmoke
            | Kind::Crit
            | Kind::DragonBreath
            | Kind::TrialSpawnerDetection
            | Kind::TrialSpawnerDetectionOminous => self.quad_size * (grown * 32.0).clamp(0.0, 1.0),
            Kind::Lava => self.quad_size * (1.0 - grown * grown),
            Kind::Flame | Kind::SmallFlame | Kind::SoulFireFlame => {
                self.quad_size * (1.0 - grown * grown * 0.5)
            }
            Kind::CosySmoke
            | Kind::SignalSmoke
            | Kind::Poof
            | Kind::Explosion
            | Kind::ExplosionEmitter
            | Kind::HappyVillager
            | Kind::Rain
            | Kind::Snowflake
            | Kind::EndRod
            | Kind::Totem
            | Kind::TrialOmen
            | Kind::OminousSpawning
            | Kind::VaultConnection
            | Kind::SculkSoul
            | Kind::SculkCharge
            | Kind::SculkChargePop => self.quad_size,
            Kind::Heart => self.quad_size * (grown * 32.0).clamp(0.0, 1.0),
            Kind::AngryVillager => self.quad_size * (grown * 32.0).clamp(0.0, 1.0),
            Kind::Splash => self.quad_size,
            Kind::Spit => self.quad_size,
        }
    }

    fn drift<S: Scene + ?Sized>(
        &mut self,
        scene: &S,
        cache: &mut HashMap<BlockPos, Boxes>,
        random: &mut LegacyRandom,
    ) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime || self.alpha <= 0.0 {
            return false;
        }
        let mut sway = || {
            let sign = if random.next_boolean() { 1.0 } else { -1.0 };
            f64::from(random.next_float() / 5000.0 * sign)
        };
        self.velocity.x += sway();
        self.velocity.z += sway();
        self.velocity.y -= f64::from(self.gravity);
        self.travel(self.velocity, scene, cache);
        if self.age >= self.lifetime - CAMPFIRE_FADE_TICKS && self.alpha > 0.01 {
            self.alpha -= 0.015;
        }
        true
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

fn is_air(block: Option<&crate::scene::Block>) -> bool {
    block.is_none_or(|block| {
        block.id.namespace == "minecraft"
            && matches!(block.id.path.as_str(), "air" | "cave_air" | "void_air")
    })
}

fn normalize(v: DVec3) -> DVec3 {
    let length = v.length();
    if length < f64::from(1.0e-5_f32) {
        DVec3::ZERO
    } else {
        v / length
    }
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
    let big_smoke = (0..BIG_SMOKE_SPRITES).map(|index| format!("big_smoke_{index}"));
    let explosion = (0..EXPLOSION_SPRITES).map(|index| format!("explosion_{index}"));
    let glitter = (0..GLITTER_SPRITES).map(|index| format!("glitter_{index}"));
    let detection = (0..TRIAL_DETECTION_SPRITES).flat_map(|index| {
        [
            format!("trial_spawner_detection_{index}"),
            format!("trial_spawner_detection_ominous_{index}"),
        ]
    });
    let souls = (0..SCULK_SOUL_SPRITES).map(|index| format!("sculk_soul_{index}"));
    let charges = (0..SCULK_CHARGE_SPRITES).map(|index| format!("sculk_charge_{index}"));
    let pops = (0..SCULK_CHARGE_POP_SPRITES).map(|index| format!("sculk_charge_pop_{index}"));
    ["lava", "glint", "flame", "soul_fire_flame", "critical_hit", "heart", "angry", "trial_omen", "ominous_spawning", "vault_connection"]
        .into_iter()
        .map(str::to_owned)
        .chain(detection)
        .chain(big_smoke)
        .chain(explosion)
        .chain(glitter)
        .chain(souls)
        .chain(charges)
        .chain(pops)
        .filter_map(|name| ResourceId::parse(&format!("minecraft:particle/{name}")).ok())
        .collect()
}

fn facing_value(facing: &str) -> usize {
    match facing {
        "south" => 0,
        "west" => 1,
        "east" => 3,
        _ => 2,
    }
}

pub struct AmbientParticles {
    particles: Vec<Particle>,
    random: LegacyRandom,
    campfires: (Option<u64>, Vec<BlockPos>),
    blasts: Vec<(DVec3, f32, usize)>,
    temperature: minecraftoss_generator::temperature::BiomeTemperature,
    rain_sound_time: i32,
    emitters: Vec<(u128, i32)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Body {
    pub feet: DVec3,
    pub width: f64,
    pub height: f64,
}

impl AmbientParticles {
    pub fn new(seed: u64) -> Self {
        Self {
            particles: Vec::new(),
            random: LegacyRandom::new(seed),
            campfires: (None, Vec::new()),
            blasts: Vec::new(),
            temperature: minecraftoss_generator::temperature::BiomeTemperature::new(),
            rain_sound_time: 0,
            emitters: Vec::new(),
        }
    }

    pub fn totem_emitter(&mut self, key: u128, body: Body) {
        self.emit_totem(body);
        self.emitters.push((key, 1));
    }

    pub fn tick_emitters(&mut self, body: impl Fn(u128) -> Option<Body>) {
        let mut emitters = std::mem::take(&mut self.emitters);
        emitters.retain_mut(|(key, life)| {
            let Some(body) = body(*key) else {
                return false;
            };
            self.emit_totem(body);
            *life += 1;
            *life < TOTEM_EMITTER_TICKS
        });
        emitters.append(&mut self.emitters);
        self.emitters = emitters;
    }

    fn emit_totem(&mut self, body: Body) {
        for _ in 0..TOTEM_TRIES {
            let mut unit = || f64::from(self.random.next_float() * 2.0 - 1.0);
            let (x, y, z) = (unit(), unit(), unit());
            if x * x + y * y + z * z > 1.0 {
                continue;
            }
            let at = body.feet
                + DVec3::new(
                    body.width * (x / 4.0),
                    body.height * (0.5 + y / 4.0),
                    body.width * (z / 4.0),
                );
            self.add(Kind::Totem, at, DVec3::new(x, y + 0.2, z));
        }
    }

    pub fn precipitation_at(
        &self,
        scene: &HandcraftedScene,
        pos: crate::scene::BlockPos,
    ) -> crate::rain::Precipitation {
        crate::rain::precipitation_at(scene, &self.temperature, pos)
    }

    pub fn rain_tick(
        &mut self,
        scene: &HandcraftedScene,
        camera: DVec3,
        rain_level: f32,
    ) -> Vec<crate::display_ticks::AmbientSound> {
        if rain_level <= 0.0 {
            return Vec::new();
        }
        let height = |x, z| crate::rain::motion_blocking_height(scene, x, z);
        let temperature = &self.temperature;
        let rains = |pos| crate::rain::rains_at(scene, temperature, pos);
        let world = crate::rain::RainWorld {
            height: &height,
            min_y: scene.vertical_range().start,
            rains_at: &rains,
        };
        let tick = crate::rain::rain_tick(
            scene,
            &world,
            camera,
            rain_level,
            &mut self.random,
            &mut self.rain_sound_time,
        );
        for (kind, at, motion) in tick.particles {
            self.add(kind, at, motion);
        }
        tick.sounds
    }

    pub fn explosion(&mut self, center: DVec3, radius: f32, blocks: usize, interacts: bool) {
        if radius < SMALL_BLAST_RADIUS || !interacts {
            self.add(Kind::Explosion, center, DVec3::new(1.0, 0.0, 0.0));
        } else {
            self.add(Kind::ExplosionEmitter, center, DVec3::ZERO);
        }
        if blocks > 0 {
            self.blasts.push((center, radius, blocks));
        }
    }

    fn track_blasts<S: Scene + ?Sized>(&mut self, scene: &S) {
        let blasts = std::mem::take(&mut self.blasts);
        let total: usize = blasts.iter().map(|blast| blast.2).sum();
        for _ in 0..total.min(MAX_BLAST_PARTICLES) {
            let mut pick = self.random.next_int(total as u32) as usize;
            let Some(&(center, radius, _)) = blasts.iter().find(|blast| {
                if pick < blast.2 {
                    return true;
                }
                pick -= blast.2;
                false
            }) else {
                continue;
            };
            self.blast_particle(scene, center, radius);
        }
    }

    fn blast_particle<S: Scene + ?Sized>(&mut self, scene: &S, center: DVec3, radius: f32) {
        let random = &mut self.random;
        let mut unit = || f64::from(random.next_float() * 2.0 - 1.0);
        let direction = normalize(DVec3::new(unit(), unit(), unit()));
        let distance = f64::from(random.next_float()).cbrt() as f32 * radius;
        let offset = direction * f64::from(distance);
        let reached = (center + offset).floor();
        let cell = (reached.x as i32, reached.y as i32, reached.z as i32);
        if !is_air(scene.block(cell)) {
            return;
        }
        let speed =
            0.5 / (distance / radius + 0.1) * random.next_float() * random.next_float() + 0.3;
        let (kind, scaling) = if random.next_int(2) == 0 {
            (Kind::Poof, 0.5)
        } else {
            (Kind::Smoke, 1.0)
        };
        self.add(
            kind,
            center + offset * scaling,
            direction * f64::from(speed),
        );
    }

    pub fn campfire_tick(
        &mut self,
        scene: &HandcraftedScene,
        food: impl Fn(BlockPos) -> [bool; 4],
    ) {
        if self.campfires.0 != Some(scene.revision()) {
            self.campfires = (Some(scene.revision()), scene.campfires());
        }
        let campfires = std::mem::take(&mut self.campfires.1);
        for &pos in &campfires {
            let Some(block) =
                Scene::block(scene, pos).filter(|block| crate::campfire::is_lit(block))
            else {
                continue;
            };
            let signal = block
                .properties
                .get("signal_fire")
                .is_some_and(|v| v == "true");
            let facing = block
                .properties
                .get("facing")
                .map_or("north", String::as_str);
            self.campfire(pos, signal, facing, food(pos));
        }
        self.campfires.1 = campfires;
    }

    pub fn campfire(&mut self, pos: BlockPos, signal: bool, facing: &str, food: [bool; 4]) {
        if self.random.next_float() < 0.11 {
            let mut puffs = 0;
            while puffs < self.random.next_int(2) + 2 {
                self.campfire_puff(pos, signal);
                puffs += 1;
            }
        }
        let facing = facing_value(facing);
        for (slot, cooking) in food.into_iter().enumerate() {
            if !cooking || self.random.next_float() >= 0.2 {
                continue;
            }
            let direction = (slot + facing) % 4;
            let (sx, sz) = SIDE_STEPS[direction];
            let (cx, cz) = SIDE_STEPS[(direction + 1) % 4];
            let offset = |step: i32| f64::from(step as f32 * CAMPFIRE_SLOT_OFFSET);
            let x = f64::from(pos.0) + 0.5 - offset(sx) + offset(cx);
            let y = f64::from(pos.1) + 0.5;
            let z = f64::from(pos.2) + 0.5 - offset(sz) + offset(cz);
            for _ in 0..4 {
                let rise = DVec3::new(0.0, 5.0e-4, 0.0);
                self.add(Kind::Smoke, DVec3::new(x, y, z), rise);
            }
        }
    }

    pub fn douse(&mut self, pos: BlockPos, signal: bool) {
        for _ in 0..DOUSE_PUFFS {
            self.campfire_puff(pos, signal);
            let random = &mut self.random;
            let sign = |random: &mut LegacyRandom| if random.next_boolean() { 1.0 } else { -1.0 };
            let x = f64::from(pos.0) + 0.5 + random.next_double() / 4.0 * sign(random);
            let y = f64::from(pos.1) + 0.4;
            let z = f64::from(pos.2) + 0.5 + random.next_double() / 4.0 * sign(random);
            self.add(
                Kind::Smoke,
                DVec3::new(x, y, z),
                DVec3::new(0.0, 0.005, 0.0),
            );
        }
    }

    fn campfire_puff(&mut self, pos: BlockPos, signal: bool) {
        let random = &mut self.random;
        let sign = |random: &mut LegacyRandom| if random.next_boolean() { 1.0 } else { -1.0 };
        let x = f64::from(pos.0) + 0.5 + random.next_double() / 3.0 * sign(random);
        let y = f64::from(pos.1) + random.next_double() + random.next_double();
        let z = f64::from(pos.2) + 0.5 + random.next_double() / 3.0 * sign(random);
        let kind = if signal {
            Kind::SignalSmoke
        } else {
            Kind::CosySmoke
        };
        self.add(kind, DVec3::new(x, y, z), DVec3::new(0.0, 0.07, 0.0));
    }

    pub fn puffs(&mut self, kind: Kind, at: DVec3, count: usize, spread: f64) {
        for _ in 0..count {
            let random = &mut self.random;
            let offset = DVec3::new(
                random.next_gaussian() * spread,
                random.next_gaussian() * spread,
                random.next_gaussian() * spread,
            );
            for _ in 0..3 {
                random.next_gaussian();
            }
            self.add(kind, at + offset, DVec3::ZERO);
        }
    }

    pub fn add_breath(&mut self, at: DVec3, motion: DVec3, power: f32) {
        let particle = Particle::dragon_breath(at, motion, power, &mut self.random);
        self.particles.push(particle);
        if self.particles.len() > CAP {
            self.particles.remove(0);
        }
    }

    pub fn breath_cloud(&mut self, feet: DVec3, radius: f32, waiting: bool) {
        if waiting && self.random.next_boolean() {
            return;
        }
        let (count, radius) = if waiting {
            (WAITING_CLOUD_PUFFS, WAITING_CLOUD_RADIUS)
        } else {
            ((std::f32::consts::PI * radius * radius).ceil() as usize, radius)
        };
        for _ in 0..count {
            let angle = self.random.next_float() * std::f32::consts::TAU;
            let distance = self.random.next_float().sqrt() * radius;
            let at = DVec3::new(
                feet.x + f64::from(mth::cos(f64::from(angle)) * distance),
                feet.y,
                feet.z + f64::from(mth::sin(f64::from(angle)) * distance),
            );
            let motion = if waiting {
                DVec3::ZERO
            } else {
                let x = (0.5 - self.random.next_double()) * 0.15;
                let z = (0.5 - self.random.next_double()) * 0.15;
                DVec3::new(x, f64::from(0.01_f32), z)
            };
            self.add_breath(at, motion, 1.0);
        }
    }

    pub fn breath_burst(&mut self, at: DVec3) {
        let block = at.floor();
        for _ in 0..BURST_PARTICLES {
            let power = self.random.next_float() * 4.0;
            let angle = self.random.next_float() * std::f32::consts::TAU;
            let x = f64::from(mth::cos(f64::from(angle)) * power);
            let y = 0.01 + self.random.next_double() * 0.5;
            let z = f64::from(mth::sin(f64::from(angle)) * power);
            let start = DVec3::new(block.x + x * 0.1, block.y + 0.3, block.z + z * 0.1);
            self.add_breath(start, DVec3::new(x, y, z), power);
        }
    }

    pub fn mouth_breath(&mut self, breath: &crate::ender_dragon::DragonBreath) {
        let look = breath.look;
        let sideways = f64::from(0.08_f32);
        for _ in 0..MOUTH_POINTS {
            let x = breath.head.x + self.random.next_gaussian() / 2.0;
            let y = breath.head.y + self.random.next_gaussian() / 2.0;
            let z = breath.head.z + self.random.next_gaussian() / 2.0;
            let at = DVec3::new(x, y, z);
            match breath.kind {
                crate::ender_dragon::BreathKind::Landing => {
                    let motion = DVec3::new(
                        -look.x * sideways + breath.motion.x,
                        -look.y * f64::from(0.3_f32) + breath.motion.y,
                        -look.z * sideways + breath.motion.z,
                    );
                    self.add_breath(at, motion, 1.0);
                }
                crate::ender_dragon::BreathKind::Flame => {
                    for speed in 0..FLAME_SPEEDS {
                        let speed = speed as f64;
                        let motion = DVec3::new(
                            -look.x * sideways * speed,
                            -look.y * f64::from(0.6_f32),
                            -look.z * sideways * speed,
                        );
                        self.add_breath(at, motion, 1.0);
                    }
                }
            }
        }
    }

    pub fn breath_trail(&mut self, at: DVec3) {
        self.add_breath(at + DVec3::new(0.0, 0.5, 0.0), DVec3::ZERO, 1.0);
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

    pub fn kinds_at(&self) -> impl Iterator<Item = (Kind, DVec3)> + '_ {
        self.particles.iter().map(|particle| (particle.kind, particle.position))
    }

    pub fn add(&mut self, kind: Kind, at: DVec3, motion: DVec3) {
        let random = &mut self.random;
        let particle = match kind {
            Kind::Smoke | Kind::LargeSmoke => Particle::smoke(kind, at, motion, random),
            Kind::Lava => Particle::lava(at, random),
            Kind::CosySmoke | Kind::SignalSmoke => {
                Particle::campfire_smoke(kind, at, motion, random)
            }
            Kind::Poof => Particle::poof(at, motion, random),
            Kind::Explosion => Particle::huge_explosion(at, motion.x, random),
            Kind::ExplosionEmitter => Particle::emitter(at, random),
            Kind::HappyVillager => Particle::happy_villager(at, motion, random),
            Kind::Rain => Particle::rain(at, random),
            Kind::Flame | Kind::SmallFlame | Kind::SoulFireFlame => {
                Particle::flame(kind, at, motion, random)
            }
            Kind::TrialSpawnerDetection | Kind::TrialSpawnerDetectionOminous => {
                Particle::trial_detection(kind, at, motion, random)
            }
            Kind::TrialOmen => Particle::spell(kind, at, motion, random),
            Kind::OminousSpawning => Particle::ominous_spawning(at, motion, random),
            Kind::VaultConnection => Particle::vault_connection(at, motion, random),
            Kind::Snowflake => Particle::snowflake(at, motion, random),
            Kind::EndRod => Particle::end_rod(at, motion, random),
            Kind::Crit => Particle::crit(at, motion, random),
            Kind::DragonBreath => Particle::dragon_breath(at, motion, 1.0, random),
            Kind::Totem => Particle::totem(at, motion, random),
            Kind::Heart => Particle::heart(kind, at, random),
            Kind::AngryVillager => Particle::heart(kind, at + DVec3::new(0.0, 0.5, 0.0), random),
            Kind::Splash => Particle::splash(at, motion, random),
            Kind::SculkSoul => Particle::sculk_soul(at, motion, random),
            Kind::SculkCharge | Kind::SculkChargePop => Particle::sculk_charge(kind, at, motion, random),
            Kind::Spit => Particle::spit(at, motion, random),
        };
        self.particles.push(particle);
        if self.particles.len() > CAP {
            self.particles.remove(0);
        }
    }

    pub fn ominous_spawning(&mut self, at: DVec3) {
        let count = self.random.next_int(3) + 1;
        for _ in 0..count {
            let mut spread = || OMINOUS_SPAWNING_SPREAD * (self.random.next_gaussian() - self.random.next_gaussian());
            let x = spread();
            let y = spread();
            let offset = DVec3::new(x, y, spread());
            self.add(Kind::OminousSpawning, at, offset);
        }
    }

    pub fn tick<S: Scene + ?Sized>(&mut self, scene: &S) {
        self.track_blasts(scene);
        let mut cache = HashMap::new();
        let mut spawned = Vec::new();
        let random = &mut self.random;
        self.particles.retain_mut(|particle| match particle.kind {
            Kind::ExplosionEmitter => {
                let size = f64::from(particle.age as f32 / particle.lifetime as f32);
                for _ in 0..EMITTER_FLASHES {
                    let mut spread = || (random.next_double() - random.next_double()) * 4.0;
                    let at = particle.position + DVec3::new(spread(), spread(), spread());
                    spawned.push((Kind::Explosion, at, DVec3::new(size, 0.0, 0.0)));
                }
                particle.age += 1;
                particle.age != particle.lifetime
            }
            Kind::Explosion => particle.hold(),
            Kind::HappyVillager => particle.linger(),
            Kind::Rain => particle.drip(scene, &mut cache, random),
            Kind::Flame | Kind::SmallFlame | Kind::SoulFireFlame => particle.rise(),
            Kind::Snowflake => {
                let alive = particle.tick(scene, &mut cache);
                let (side, down) = (f64::from(0.95_f32), f64::from(0.9_f32));
                particle.velocity *= DVec3::new(side, down, side);
                alive
            }
            Kind::CosySmoke | Kind::SignalSmoke => particle.drift(scene, &mut cache, random),
            Kind::EndRod | Kind::Crit | Kind::TrialOmen => particle.coast(),
            Kind::OminousSpawning => particle.fly_home(),
            Kind::VaultConnection => particle.fly_to_keyhole(),
            Kind::DragonBreath => particle.breathe(),
            Kind::Heart => particle.coast(),
            Kind::AngryVillager => particle.coast(),
            Kind::Splash => particle.drip(scene, &mut cache, random),
            Kind::Totem => {
                let alive = particle.tick(scene, &mut cache);
                particle.fade_after_half_life();
                alive
            }
            _ => {
                let alive = particle.tick(scene, &mut cache);
                if alive && particle.kind == Kind::Lava {
                    let aged = particle.age as f32 / particle.lifetime as f32;
                    if random.next_float() > aged {
                        spawned.push((Kind::Smoke, particle.position, particle.velocity));
                    }
                }
                alive
            }
        });
        for (kind, at, motion) in spawned {
            self.add(kind, at, motion);
        }
    }

    pub fn append_mesh(
        &self,
        opaque: &mut ChunkMesh,
        translucent: &mut ChunkMesh,
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
            let (sky, block) = match particle.kind {
                Kind::Explosion | Kind::EndRod | Kind::Totem => (FULL_BRIGHT, FULL_BRIGHT),
                Kind::Lava | Kind::TrialSpawnerDetection | Kind::TrialSpawnerDetectionOminous | Kind::OminousSpawning | Kind::VaultConnection | Kind::SculkSoul | Kind::SculkCharge | Kind::SculkChargePop => (f32::from(light.get(cell)), FULL_BRIGHT),
                Kind::Flame | Kind::SmallFlame | Kind::SoulFireFlame => (f32::from(light.get(cell)), particle.glow(f32::from(light.get_block(cell)), partial)),
                _ => (f32::from(light.get(cell)), f32::from(light.get_block(cell))),
            };
            let size = particle.size_at(partial);
            let center = center.as_vec3();
            let [r, g, b] = particle.color;
            let color = [r, g, b, particle.alpha];
            let mesh = if particle.translucent() {
                &mut *translucent
            } else {
                &mut *opaque
            };
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
    fn a_flame_hardly_moves_shrinks_and_glows_as_it_ages() {
        let stone = Blocks(HashMap::from([((0, 64, 0), Block::new("minecraft:stone"))]));
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![12], vec![3]);
        let mut crossed = false;
        for seed in 0..200 {
            let mut particles = AmbientParticles::new(seed);
            let start = DVec3::new(0.5, 63.99, 0.5);
            particles.add(Kind::Flame, start, DVec3::ZERO);
            let flame = particles.particles[0].clone();
            assert!((12..=44).contains(&flame.lifetime), "{}", flame.lifetime);
            assert!((flame.position - start).abs().max_element() <= 0.05);
            assert_eq!(flame.previous, start);
            assert_eq!(flame.size_at(0.0), flame.quad_size);
            assert_eq!(flame.glow(3.0, 0.0), 3.0);
            let mut end = flame.clone();
            end.age = end.lifetime;
            assert_eq!(end.size_at(0.0), flame.quad_size * 0.5);
            assert_eq!(end.glow(3.0, 0.0), 15.0);
            let mut rise = 0.0_f64;
            let mut lived = 0;
            while particles.len() == 1 {
                rise = rise.max((particles.particles[0].position - flame.position).length());
                crossed |= flame.position.y < 64.0 && particles.particles[0].position.y > 64.0;
                particles.tick(&stone);
                lived += 1;
            }
            assert!(rise < 0.1, "{rise}");
            assert_eq!(lived, flame.lifetime + 1);
        }
        assert!(crossed);
        let mut particles = AmbientParticles::new(1);
        particles.add(Kind::Flame, DVec3::new(0.5, 64.5, 0.5), DVec3::ZERO);
        let mut opaque = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let atlas = crate::mesh::make_atlas(&crate::pack::PackStack::open(Vec::new()).unwrap(), textures()).unwrap();
        particles.append_mesh(&mut opaque, &mut translucent, &atlas, Vec3::Z, 0.0, &light);
        assert_eq!(opaque.vertices.len(), 4);
        assert!(translucent.vertices.is_empty());
        assert!(opaque.vertices.iter().all(|v| v.sky_light == 12.0 && v.block_light == 3.0));
        let mut flame = particles.particles[0].clone();
        flame.age = flame.lifetime / 2;
        assert!(flame.glow(3.0, 0.0) > 3.0 && flame.glow(3.0, 0.0) < 15.0);
    }

    #[test]
    fn small_and_soul_flames_are_flames_with_their_own_size_and_sprite() {
        let start = DVec3::new(0.5, 64.5, 0.5);
        let mut made = Vec::new();
        for kind in [Kind::Flame, Kind::SmallFlame, Kind::SoulFireFlame] {
            let mut particles = AmbientParticles::new(7);
            particles.add(kind, start, DVec3::ZERO);
            made.push(particles.particles[0].clone());
        }
        let (flame, small, soul) = (&made[0], &made[1], &made[2]);
        assert_eq!(small.quad_size, flame.quad_size * 0.5);
        assert_eq!(soul.quad_size, flame.quad_size);
        for particle in [small, soul] {
            assert_eq!(particle.lifetime, flame.lifetime);
            assert_eq!(particle.position, flame.position);
            assert_eq!(particle.velocity, flame.velocity);
            assert_eq!(particle.glow(3.0, 0.0), 3.0);
        }
        let sprite = |particle: &Particle| particle.sprite().unwrap().key();
        assert_eq!(sprite(small), "minecraft:particle/flame");
        assert_eq!(sprite(soul), "minecraft:particle/soul_fire_flame");
        assert!(textures().contains(&ResourceId::parse("minecraft:particle/soul_fire_flame").unwrap()));
        let mut particles = AmbientParticles::new(7);
        particles.add(Kind::SoulFireFlame, start, DVec3::ZERO);
        let lifetime = particles.particles[0].lifetime;
        let mut lived = 0;
        while !particles.is_empty() {
            particles.tick(&open());
            lived += 1;
        }
        assert_eq!(lived, lifetime + 1);
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
    fn campfire_smoke_rises_for_its_life_and_fades_out_over_the_last_three_seconds() {
        let mut particles = AmbientParticles::new(7);
        particles.add(
            Kind::CosySmoke,
            DVec3::new(0.5, 65.0, 0.5),
            DVec3::new(0.0, 0.07, 0.0),
        );
        particles.add(
            Kind::SignalSmoke,
            DVec3::new(0.5, 65.0, 0.5),
            DVec3::new(0.0, 0.07, 0.0),
        );
        let cosy = particles.particles[0].clone();
        let signal = particles.particles[1].clone();
        assert!((80..130).contains(&cosy.lifetime) && (280..330).contains(&signal.lifetime));
        assert_eq!((cosy.alpha, signal.alpha), (0.9, 0.95));
        assert!(cosy.quad_size >= 0.3 - 1e-6 && cosy.quad_size <= 0.6 + 1e-6);
        assert!(cosy.translucent() && cosy.sprite < 12);
        assert_eq!(cosy.velocity.x, 0.0);
        assert!(cosy.velocity.y >= 0.07 && cosy.velocity.y <= 0.07 + 0.002);
        for _ in 0..cosy.lifetime - CAMPFIRE_FADE_TICKS - 1 {
            particles.tick(&open());
        }
        let early = particles.particles[0].clone();
        assert_eq!(early.alpha, 0.9);
        assert!(early.position.y > 65.0 + 0.07 * 15.0);
        particles.tick(&open());
        assert!(particles.particles[0].alpha < 0.9);
        for _ in 0..CAMPFIRE_FADE_TICKS + 1 {
            particles.tick(&open());
        }
        assert_eq!(particles.len(), 1);
        assert_eq!(particles.particles[0].kind, Kind::SignalSmoke);
    }

    #[test]
    fn a_campfire_puffs_two_or_three_smokes_one_tick_in_nine() {
        let mut particles = AmbientParticles::new(8);
        let mut puffs = 0;
        let mut ticks_with_smoke = 0;
        for _ in 0..10_000 {
            let before = particles.len();
            particles.campfire((4, 70, -3), false, "north", [false; 4]);
            let added = particles.len() - before;
            if added > 0 {
                assert!((2..=3).contains(&added), "{added}");
                ticks_with_smoke += 1;
                puffs += added;
            }
            if particles.len() > 1000 {
                particles.clear();
            }
        }
        assert!(
            (950..1250).contains(&ticks_with_smoke),
            "{ticks_with_smoke}"
        );
        assert!(puffs > 2 * ticks_with_smoke && puffs < 3 * ticks_with_smoke);
    }

    #[test]
    fn a_doused_campfire_bursts_into_twenty_puffs_and_twenty_low_smokes() {
        for (signal, kind) in [(false, Kind::CosySmoke), (true, Kind::SignalSmoke)] {
            let mut particles = AmbientParticles::new(10);
            particles.douse((4, 70, -3), signal);
            assert_eq!(particles.len(), 40);
            let puffs: Vec<_> = particles
                .particles
                .iter()
                .filter(|p| p.kind == kind)
                .collect();
            let smokes: Vec<_> = particles
                .particles
                .iter()
                .filter(|p| p.kind == Kind::Smoke)
                .collect();
            assert_eq!((puffs.len(), smokes.len()), (20, 20));
            for puff in puffs {
                assert!((4.5 - 1.0 / 3.0..=4.5 + 1.0 / 3.0).contains(&puff.position.x));
                assert!((70.0..72.0).contains(&puff.position.y));
            }
            for smoke in smokes {
                assert!((4.25..=4.75).contains(&smoke.position.x));
                assert!((-2.75..=-2.25).contains(&smoke.position.z));
                assert_eq!(smoke.position.y, 70.4);
            }
        }
    }

    #[test]
    fn campfire_smoke_starts_over_the_fire_and_a_signal_fire_sends_signal_smoke() {
        let mut particles = AmbientParticles::new(9);
        for _ in 0..2000 {
            particles.campfire((4, 70, -3), true, "north", [false; 4]);
        }
        assert!(!particles.is_empty());
        for particle in &particles.particles {
            assert_eq!(particle.kind, Kind::SignalSmoke);
            let start = particle.previous;
            assert!(start.x > 4.5 - 1.0 / 3.0 && start.x < 4.5 + 1.0 / 3.0);
            assert!(start.z > -2.5 - 1.0 / 3.0 && start.z < -2.5 + 1.0 / 3.0);
            assert!((70.0..72.0).contains(&start.y));
        }
    }

    #[test]
    fn food_on_a_campfire_smokes_four_puffs_from_its_corner() {
        let mut particles = AmbientParticles::new(10);
        let mut seen = std::collections::BTreeSet::new();
        for _ in 0..400 {
            particles.campfire((0, 64, 0), false, "east", [true, false, false, true]);
            for particle in particles.particles.iter().filter(|p| p.kind == Kind::Smoke) {
                let at = particle.previous;
                assert_eq!(at.y, 64.5);
                seen.insert(((at.x * 16.0).round() as i32, (at.z * 16.0).round() as i32));
            }
            particles.clear();
        }
        assert_eq!(seen, std::collections::BTreeSet::from([(3, 13), (13, 13)]));
    }

    #[test]
    fn only_lit_campfires_in_the_scene_smoke() {
        let mut scene = HandcraftedScene::default();
        scene.set(
            (2, 64, 2),
            Some(Block::new("minecraft:campfire").with("lit", "true")),
        );
        scene.set(
            (9, 64, 2),
            Some(Block::new("minecraft:campfire").with("lit", "false")),
        );
        scene.set((5, 64, 5), Some(Block::new("minecraft:stone")));
        let mut particles = AmbientParticles::new(11);
        for _ in 0..200 {
            particles.campfire_tick(&scene, |_| [false; 4]);
        }
        assert!(!particles.is_empty());
        assert!(
            particles
                .particles
                .iter()
                .all(|p| (p.previous.x - 2.5).abs() < 0.34)
        );
        scene.set(
            (2, 64, 2),
            Some(Block::new("minecraft:campfire").with("lit", "false")),
        );
        particles.clear();
        for _ in 0..200 {
            particles.campfire_tick(&scene, |_| [false; 4]);
        }
        assert!(particles.is_empty());
    }

    fn count(particles: &AmbientParticles, kind: Kind) -> usize {
        particles
            .particles
            .iter()
            .filter(|p| p.kind == kind)
            .count()
    }

    #[test]
    fn a_big_blast_seeds_six_flashes_a_tick_for_eight_ticks() {
        let mut particles = AmbientParticles::new(12);
        let center = DVec3::new(0.5, 64.0, 0.5);
        particles.explosion(center, 4.0, 0, true);
        assert_eq!(count(&particles, Kind::ExplosionEmitter), 1);
        let mut flashes = 0;
        for tick in 0..EMITTER_TICKS {
            let before = count(&particles, Kind::Explosion);
            particles.tick(&open());
            let born: Vec<_> = particles
                .particles
                .iter()
                .filter(|p| p.kind == Kind::Explosion && p.age == 0)
                .collect();
            assert_eq!(born.len(), 6, "tick {tick} after {before}");
            let size = 2.0 * (1.0 - tick as f32 / 8.0 * 0.5);
            for flash in born {
                assert!((flash.quad_size - size).abs() < 1e-6);
                assert!((flash.position - center).abs().max_element() < 4.0);
                assert!((6..10).contains(&flash.lifetime));
            }
            flashes += 6;
        }
        assert_eq!(flashes, 48);
        assert_eq!(count(&particles, Kind::ExplosionEmitter), 0);
        for _ in 0..10 {
            particles.tick(&open());
        }
        assert_eq!(count(&particles, Kind::Explosion), 0);
    }

    #[test]
    fn a_small_or_harmless_blast_is_one_flash() {
        let mut particles = AmbientParticles::new(13);
        particles.explosion(DVec3::ZERO, 1.0, 0, true);
        particles.explosion(DVec3::ZERO, 3.0, 0, false);
        assert_eq!(count(&particles, Kind::Explosion), 2);
        assert!(particles.particles.iter().all(|p| p.quad_size == 1.0));
        let flash = &particles.particles[0];
        assert_eq!(
            flash.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/explosion_0")
        );
    }

    #[test]
    fn a_blast_scatters_poofs_and_smoke_through_the_air_it_reached() {
        let mut particles = AmbientParticles::new(14);
        let center = DVec3::new(0.5, 64.0, 0.5);
        particles.explosion(center, 3.0, 2000, false);
        particles.track_blasts(&open());
        let scattered = count(&particles, Kind::Poof) + count(&particles, Kind::Smoke);
        assert_eq!(scattered, 512);
        assert!(count(&particles, Kind::Poof) > 200 && count(&particles, Kind::Smoke) > 200);
        assert!(
            particles
                .particles
                .iter()
                .filter(|p| p.kind != Kind::Explosion)
                .all(|p| (p.position - center).length() <= 3.0 + 1e-6)
        );
        assert!(particles.blasts.is_empty());
        let mut few = AmbientParticles::new(15);
        few.explosion(center, 3.0, 40, true);
        few.track_blasts(&open());
        assert_eq!(count(&few, Kind::Poof) + count(&few, Kind::Smoke), 40);
    }

    #[test]
    fn blast_particles_only_start_in_air() {
        let mut stone = HashMap::new();
        for x in -4_i32..=4 {
            for y in 60_i32..=68 {
                for z in -4_i32..=4 {
                    if x.abs() > 1 || (y - 64).abs() > 1 || z.abs() > 1 {
                        stone.insert((x, y, z), Block::new("minecraft:stone"));
                    }
                }
            }
        }
        let scene = Blocks(stone);
        let mut particles = AmbientParticles::new(16);
        particles.explosion(DVec3::new(0.5, 64.5, 0.5), 4.0, 500, true);
        particles.track_blasts(&scene);
        let scattered: Vec<_> = particles
            .particles
            .iter()
            .filter(|p| matches!(p.kind, Kind::Poof | Kind::Smoke))
            .collect();
        assert!(
            scattered.len() > 10 && scattered.len() < 150,
            "{}",
            scattered.len()
        );
        for particle in scattered {
            let at = particle.position;
            assert!(
                (-1.0..2.0).contains(&at.x)
                    && (63.0..66.0).contains(&at.y)
                    && (-1.0..2.0).contains(&at.z),
                "{at}"
            );
        }
    }

    #[test]
    fn happy_villager_sparkles_hang_in_the_air_and_count_down() {
        let mut particles = AmbientParticles::new(17);
        let at = DVec3::new(0.5, 64.5, 0.5);
        particles.add(Kind::HappyVillager, at, DVec3::new(0.0, 0.01, 0.0));
        let sparkle = particles.particles[0].clone();
        assert_eq!(sparkle.color, [1.0; 3]);
        assert!((20..=100).contains(&sparkle.lifetime));
        assert!(sparkle.velocity.length() < 0.01);
        assert_eq!(
            sparkle.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/glint")
        );
        let scene = Blocks(HashMap::from([((0, 64, 0), Block::new("minecraft:stone"))]));
        for _ in 0..sparkle.lifetime {
            particles.tick(&scene);
        }
        assert_eq!(particles.len(), 1);
        assert!((particles.particles[0].position - at).length() < 0.5);
        particles.tick(&scene);
        assert!(particles.is_empty());
    }

    #[test]
    fn rain_splashes_hop_up_and_die_on_the_ground_or_in_water() {
        let mut particles = AmbientParticles::new(18);
        particles.add(Kind::Rain, DVec3::new(0.5, 65.0, 0.5), DVec3::ZERO);
        let splash = particles.particles[0].clone();
        assert!(splash.velocity.y >= 0.1 && splash.velocity.y < 0.3 + 1e-6);
        assert!(splash.velocity.x.abs() < 0.1 && splash.sprite < 4);
        assert!((8..=40).contains(&splash.lifetime));
        let scene = Blocks(HashMap::from([((0, 64, 0), Block::new("minecraft:stone"))]));
        for _ in 0..60 {
            particles.tick(&scene);
        }
        assert!(particles.is_empty());
        let water = Blocks(HashMap::from([(
            (0, 64, 0),
            Block::new("minecraft:water").with("level", "0"),
        )]));
        particles.add(Kind::Rain, DVec3::new(0.5, 64.5, 0.5), DVec3::ZERO);
        particles.tick(&water);
        assert!(
            particles.is_empty(),
            "a splash inside water is gone at once"
        );
    }

    #[test]
    fn a_heart_rises_slowly_through_blocks_for_sixteen_ticks() {
        let mut particles = AmbientParticles::new(29);
        let at = DVec3::new(0.5, 64.8, 0.5);
        particles.add(Kind::Heart, at, DVec3::new(0.3, -0.3, 0.3));
        let heart = particles.particles[0].clone();
        assert_eq!(heart.lifetime, 16);
        assert!(heart.velocity.x.abs() < 0.002 && heart.velocity.z.abs() < 0.002);
        assert!((heart.velocity.y - 0.1).abs() < 0.003, "{}", heart.velocity.y);
        assert!(heart.quad_size >= 0.15 - 1e-6 && heart.quad_size <= 0.3 + 1e-6);
        assert_eq!(
            heart.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/heart")
        );
        assert_eq!(heart.size_at(0.0), 0.0);
        assert!((heart.size_at(0.25) - heart.quad_size * 0.5).abs() < 1e-6);
        assert_eq!(heart.size_at(0.5), heart.quad_size);
        assert!(!particles_translucent(Kind::Heart));
        let scene = Blocks(HashMap::from([((0, 65, 0), Block::new("minecraft:stone"))]));
        particles.tick(&scene);
        let ticked = &particles.particles[0];
        assert!((ticked.position - at - heart.velocity).abs().max_element() < 1e-9);
        let slowed = heart.velocity * f64::from(0.86_f32);
        assert!((ticked.velocity - slowed).abs().max_element() < 1e-9);
        for _ in 1..16 {
            particles.tick(&scene);
        }
        assert_eq!(particles.len(), 1);
        let y = particles.particles[0].position.y;
        assert!(y > 65.3, "it rose into the stone: {y}");
        particles.tick(&scene);
        assert!(particles.is_empty());
        assert!(textures().iter().any(|id| id.key() == "minecraft:particle/heart"));
    }

    #[test]
    fn an_angry_cloud_starts_half_a_block_higher_and_is_white() {
        let mut particles = AmbientParticles::new(31);
        let at = DVec3::new(0.5, 64.0, 0.5);
        particles.add(Kind::AngryVillager, at, DVec3::new(0.01, 0.02, -0.01));
        let cloud = particles.particles[0].clone();
        assert_eq!(cloud.position, at + DVec3::new(0.0, 0.5, 0.0));
        assert_eq!((cloud.color, cloud.alpha, cloud.lifetime), ([1.0; 3], 1.0, 16));
        assert!((cloud.velocity.y - 0.1).abs() < 0.003, "{}", cloud.velocity.y);
        assert_eq!(
            cloud.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/angry")
        );
        assert_eq!(cloud.size_at(0.0), 0.0);
        assert_eq!(cloud.size_at(0.5), cloud.quad_size);
        assert!(!particles_translucent(Kind::AngryVillager));
        let scene = open();
        for _ in 0..16 {
            particles.tick(&scene);
        }
        assert_eq!(particles.len(), 1);
        assert!(particles.particles[0].position.y > 65.0);
        particles.tick(&scene);
        assert!(particles.is_empty());
        assert!(textures().iter().any(|id| id.key() == "minecraft:particle/angry"));
    }

    #[test]
    fn a_splash_drop_falls_at_0_04_and_pops_on_the_ground() {
        let mut particles = AmbientParticles::new(37);
        let motion = DVec3::new(0.01, -0.02, 0.03);
        particles.add(Kind::Splash, DVec3::new(0.5, 65.2, 0.5), motion);
        let drop = particles.particles[0].clone();
        assert_eq!(drop.gravity, 0.04);
        assert!(drop.velocity.y >= 0.1 && drop.velocity.y < 0.3 + 1e-6);
        assert!(drop.velocity.x.abs() < 0.1 && drop.sprite < 4);
        assert!((8..=40).contains(&drop.lifetime));
        assert_eq!(
            drop.sprite().map(|id| id.key()),
            Some(format!("minecraft:particle/splash_{}", drop.sprite))
        );
        let scene = Blocks(HashMap::from([((0, 64, 0), Block::new("minecraft:stone"))]));
        particles.tick(&scene);
        let ticked = &particles.particles[0];
        let expected = (drop.velocity.y - f64::from(0.04_f32)) * f64::from(0.98_f32);
        assert!((ticked.velocity.y - expected).abs() < 1e-9, "{}", ticked.velocity.y);
        for _ in 0..60 {
            particles.tick(&scene);
        }
        assert!(particles.is_empty(), "it pops on the stone or runs out");
        let mut sideways = AmbientParticles::new(41);
        sideways.add(Kind::Splash, DVec3::new(0.5, 70.0, 0.5), DVec3::new(0.2, 0.0, -0.1));
        assert_eq!(sideways.particles[0].velocity, DVec3::new(0.2, 0.1, -0.1));
    }

    #[test]
    fn a_spit_speck_is_a_poof_that_falls_at_0_5() {
        let at = DVec3::new(0.5, 70.0, 0.5);
        let motion = DVec3::new(0.6, 0.0, -0.3);
        let (mut spit, mut poof) = (AmbientParticles::new(43), AmbientParticles::new(43));
        spit.add(Kind::Spit, at, motion);
        poof.add(Kind::Poof, at, motion);
        let (speck, puff) = (spit.particles[0].clone(), poof.particles[0].clone());
        assert_eq!((speck.kind, speck.gravity, speck.friction), (Kind::Spit, 0.5, 0.9));
        assert_eq!((speck.velocity, speck.color, speck.quad_size, speck.lifetime), (puff.velocity, puff.color, puff.quad_size, puff.lifetime));
        assert!((speck.velocity - motion).abs().max_element() <= 0.05 + 1e-6);
        assert_eq!(speck.sprite().map(|id| id.key()).as_deref(), Some("minecraft:particle/generic_7"));
        let mut old = speck.clone();
        old.age = old.lifetime;
        assert_eq!(old.sprite().map(|id| id.key()).as_deref(), Some("minecraft:particle/generic_0"));
        assert_eq!(speck.size_at(0.0), speck.quad_size);
        assert!(!particles_translucent(Kind::Spit));
        let scene = open();
        spit.tick(&scene);
        let expected = (speck.velocity.y - 0.04 * 0.5) * f64::from(0.9_f32);
        assert!((spit.particles[0].velocity.y - expected).abs() < 1e-9, "{}", spit.particles[0].velocity.y);
        for _ in 0..10 {
            spit.tick(&scene);
        }
        assert!(spit.particles[0].position.y < at.y, "it falls: {}", spit.particles[0].position.y);
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

    #[test]
    fn snowflakes_live_and_slow_like_vanilla() {
        let mut particles = AmbientParticles::new(19);
        let motion = DVec3::new(0.08, 0.05, -0.04);
        particles.add(Kind::Snowflake, DVec3::new(0.5, 65.0, 0.5), motion);
        let flake = particles.particles[0].clone();
        assert!((18..=82).contains(&flake.lifetime), "{}", flake.lifetime);
        assert!(flake.quad_size >= 0.1 && flake.quad_size <= 0.2);
        assert!((flake.velocity - motion).abs().max_element() <= 0.05 + 1e-9);
        assert_eq!(
            flake.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/generic_7")
        );
        let scene = Blocks(HashMap::new());
        particles.tick(&scene);
        let ticked = &particles.particles[0];
        let fall = f64::from(0.04_f32 * 0.225);
        let expected = DVec3::new(
            flake.velocity.x * f64::from(0.95_f32),
            (flake.velocity.y - fall) * f64::from(0.9_f32),
            flake.velocity.z * f64::from(0.95_f32),
        );
        assert!((ticked.velocity - expected).abs().max_element() < 1e-6, "{:?}", ticked.velocity);
        let moved = flake.velocity - DVec3::new(0.0, f64::from(0.04 * 0.225_f32), 0.0);
        assert!((ticked.position - flake.position - moved).abs().max_element() < 1e-6);
        for _ in 0..flake.lifetime {
            particles.tick(&scene);
        }
        assert!(particles.is_empty());
        assert!(!particles_translucent(Kind::Snowflake));
    }

    #[test]
    fn end_rod_sparkles_drift_down_glitter_and_fade_out() {
        let mut particles = AmbientParticles::new(23);
        let at = DVec3::new(0.5, 70.0, 0.5);
        particles.add(Kind::EndRod, at, DVec3::ZERO);
        let rod = particles.particles[0].clone();
        assert!((60..=71).contains(&rod.lifetime), "{}", rod.lifetime);
        assert!(rod.quad_size >= 0.075 && rod.quad_size <= 0.15);
        assert_eq!(rod.velocity, DVec3::ZERO);
        assert_eq!(
            rod.sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/glitter_7")
        );
        assert!(particles_translucent(Kind::EndRod));
        let scene = open();
        particles.tick(&scene);
        let fall = f64::from(0.04_f32 * 0.0125);
        let ticked = &particles.particles[0];
        assert!((ticked.position.y - (70.0 - fall)).abs() < 1e-9);
        assert!((ticked.velocity.y + fall * f64::from(0.91_f32)).abs() < 1e-9);
        assert_eq!((ticked.alpha, ticked.color), (1.0, [1.0; 3]));
        for _ in 0..rod.lifetime / 2 + 4 {
            particles.tick(&scene);
        }
        let late = &particles.particles[0];
        assert!(late.alpha < 1.0 && late.alpha > 0.5, "{}", late.alpha);
        assert!(late.color[2] < 1.0 && late.color[2] > END_ROD_FADE[2] - 1e-6);
        assert!(late.sprite().unwrap().key() != "minecraft:particle/glitter_7");
        for _ in 0..rod.lifetime {
            particles.tick(&scene);
        }
        assert!(particles.is_empty());
    }

    #[test]
    fn a_sculk_soul_hangs_glowing_and_runs_through_its_eleven_frames() {
        let mut particles = AmbientParticles::new(7);
        let at = DVec3::new(4.5, 65.15, 8.5);
        particles.add(Kind::SculkSoul, at, DVec3::ZERO);
        let soul = particles.particles[0].clone();
        assert!(soul.position.distance(at) < 0.09, "{}", soul.position);
        assert!((12..=44).contains(&soul.lifetime), "{}", soul.lifetime);
        assert!(soul.velocity.length() < 0.004, "it barely drifts: {}", soul.velocity);
        assert!(soul.quad_size > 0.14 && soul.quad_size <= 0.3);
        assert!(soul.translucent());
        assert_eq!(soul.sprite().map(|id| id.path).as_deref(), Some("particle/sculk_soul_0"));
        let lifetime = soul.lifetime;
        let open = open();
        for _ in 0..lifetime - 1 {
            particles.tick(&open);
        }
        let last = particles.particles[0].sprite().map(|id| id.path);
        assert!(last.as_deref().is_some_and(|name| name.starts_with("particle/sculk_soul_")));
        assert_ne!(last.as_deref(), Some("particle/sculk_soul_0"));
        particles.tick(&open);
        particles.tick(&open);
        assert!(particles.is_empty());
        assert!(textures().iter().any(|id| id.path == "particle/sculk_soul_10"));
    }

    #[test]
    fn sculk_charges_drift_at_their_given_speed_and_glow() {
        let mut particles = AmbientParticles::new(9);
        let motion = DVec3::new(0.004, 0.0, -0.003);
        particles.add(Kind::SculkCharge, DVec3::new(2.5, 64.15, 2.5), motion);
        particles.add(Kind::SculkChargePop, DVec3::new(2.5, 64.5, 2.5), DVec3::ZERO);
        let (charge, pop) = (particles.particles[0].clone(), particles.particles[1].clone());
        assert_eq!(charge.velocity, motion);
        assert!((8..20).contains(&charge.lifetime) && (6..10).contains(&pop.lifetime));
        assert!(charge.quad_size > pop.quad_size * 0.75, "one and a half times bigger at most");
        assert!(charge.translucent() && pop.translucent());
        assert_eq!(charge.sprite().map(|id| id.path).as_deref(), Some("particle/sculk_charge_0"));
        assert_eq!(pop.sprite().map(|id| id.path).as_deref(), Some("particle/sculk_charge_pop_0"));
        assert!(textures().iter().any(|id| id.path == "particle/sculk_charge_6"));
        assert!(textures().iter().any(|id| id.path == "particle/sculk_charge_pop_3"));
    }

    #[test]
    fn crits_fly_out_slowly_and_redden() {
        let mut particles = AmbientParticles::new(29);
        particles.puffs(Kind::Crit, DVec3::new(0.5, 70.0, 0.5), 15, 0.2);
        assert_eq!(particles.len(), 15);
        for crit in &particles.particles {
            assert!((1..=10).contains(&crit.lifetime), "{}", crit.lifetime);
            assert_eq!(crit.age, 1, "it ticks once as it is made");
            assert!(crit.color[0] >= 0.6 && crit.color[0] <= 0.9);
            assert!(crit.color[2] < crit.color[1] && crit.color[1] < crit.color[0]);
            assert!((crit.position - DVec3::new(0.5, 70.0, 0.5)).length() < 2.0);
            assert_eq!(
                crit.sprite().unwrap().key(),
                "minecraft:particle/critical_hit"
            );
            assert!(!crit.translucent());
            assert!(crit.size_at(0.0) <= crit.quad_size);
        }
        let mut puffs = AmbientParticles::new(31);
        puffs.puffs(Kind::Explosion, DVec3::new(0.5, 70.0, 0.5), 2, 0.2);
        assert_eq!(puffs.len(), 2);
        assert!(puffs.particles.iter().all(|p| p.quad_size == 2.0));
        let names: Vec<String> = textures().iter().map(|id| id.key()).collect();
        assert!(names.contains(&"minecraft:particle/critical_hit".to_owned()));
        assert!(names.contains(&"minecraft:particle/glitter_0".to_owned()));
        assert!(names.contains(&"minecraft:particle/glitter_7".to_owned()));
    }

    #[test]
    fn a_breath_particle_draws_its_lifetime_size_and_colour_in_vanilla_order() {
        for seed in [3, 40, 777] {
            let mut particles = AmbientParticles::new(seed);
            particles.add_breath(DVec3::new(0.5, 70.0, 0.5), DVec3::ZERO, 1.0);
            let breath = particles.particles[0].clone();
            let mut random = LegacyRandom::new(seed);
            random.next_float();
            let quad_size = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0 * 0.75;
            let red = random.next_float() * (0.8745098 - 0.7176471) + 0.7176471;
            let blue = random.next_float() * (0.9764706 - 0.8235294) + 0.8235294;
            let lifetime = (20.0 / (f64::from(random.next_float()) * 0.8 + 0.2)) as i32;
            assert_eq!(breath.quad_size, quad_size);
            assert_eq!(breath.color, [red, 0.0, blue]);
            assert_eq!(breath.lifetime, lifetime);
            assert!((20..=100).contains(&breath.lifetime));
            assert_eq!(breath.velocity, DVec3::ZERO);
            assert!(!breath.translucent());
            assert_eq!(breath.size_at(0.0), 0.0);
            assert_eq!(breath.size_at(1.0), breath.quad_size * (1.0 / breath.lifetime as f32 * 32.0).clamp(0.0, 1.0));
            assert_eq!(breath.sprite().map(|id| id.key()).as_deref(), Some("minecraft:particle/generic_5"));
            let mut aged = breath.clone();
            aged.age = aged.lifetime - 1;
            assert_eq!(aged.sprite().map(|id| id.key()).as_deref(), Some("minecraft:particle/generic_6"));
            aged.age = aged.lifetime;
            assert_eq!(aged.sprite().map(|id| id.key()).as_deref(), Some("minecraft:particle/generic_7"));
        }
        let mut particles = AmbientParticles::new(9);
        particles.add(Kind::DragonBreath, DVec3::ZERO, DVec3::ZERO);
        assert_eq!(particles.particles[0].kind, Kind::DragonBreath);
    }

    #[test]
    fn power_scales_the_motion_like_set_power() {
        let mut particles = AmbientParticles::new(5);
        particles.add_breath(DVec3::ZERO, DVec3::new(1.0, 0.5, -2.0), 2.0);
        let tenth = f64::from(0.1_f32);
        assert_eq!(particles.particles[0].velocity, DVec3::new(2.0, (0.5 - tenth) * 2.0 + tenth, -4.0));
        particles.add_breath(DVec3::ZERO, DVec3::new(1.0, 0.0, 1.0), 1.0);
        assert_eq!(particles.particles[1].velocity, DVec3::new(1.0, 0.0, 1.0));
    }

    #[test]
    fn a_breath_particle_drifts_through_blocks_and_spreads_when_it_stops_rising() {
        let scene = Blocks(HashMap::from([
            ((0, 63, 0), Block::new("minecraft:stone")),
            ((1, 64, 0), Block::new("minecraft:stone")),
        ]));
        let mut particles = AmbientParticles::new(6);
        particles.add_breath(DVec3::new(0.5, 64.2, 0.5), DVec3::new(0.0, -0.5, 0.0), 1.0);
        particles.tick(&scene);
        particles.tick(&scene);
        let sinking = &particles.particles[0];
        assert!(sinking.position.y < 63.5, "{}", sinking.position.y);
        assert!((sinking.velocity.y + 0.5).abs() < 1e-12);
        let mut particles = AmbientParticles::new(7);
        particles.add_breath(DVec3::new(0.5, 64.5, 0.5), DVec3::new(0.2, 0.0, 0.0), 1.0);
        particles.tick(&scene);
        let spreading = &particles.particles[0];
        assert!((spreading.position - DVec3::new(0.7, 64.5, 0.5)).length() < 1e-12);
        assert_eq!(spreading.velocity.x, 0.2 * 1.1 * f64::from(0.96_f32));
        for _ in 0..5 {
            particles.tick(&scene);
        }
        assert!(particles.particles[0].position.x > 1.5, "it passes through the stone at x 1");
        let mut particles = AmbientParticles::new(8);
        particles.add_breath(DVec3::new(0.5, 64.5, 0.5), DVec3::new(0.2, 0.05, 0.0), 1.0);
        particles.tick(&scene);
        assert_eq!(particles.particles[0].velocity.x, 0.2 * f64::from(0.96_f32));
        let lifetime = particles.particles[0].lifetime;
        for _ in 1..lifetime {
            particles.tick(&scene);
        }
        assert_eq!(particles.len(), 1);
        particles.tick(&scene);
        assert!(particles.is_empty());
    }

    #[test]
    fn a_waiting_breath_cloud_puffs_two_or_none_and_an_active_one_fills_pi_r_squared() {
        let feet = DVec3::new(10.0, 64.0, -5.0);
        let mut particles = AmbientParticles::new(11);
        let mut counts = Vec::new();
        for _ in 0..40 {
            let before = particles.len();
            particles.breath_cloud(feet, 3.0, true);
            counts.push(particles.len() - before);
        }
        assert!(counts.iter().all(|&count| count == 0 || count == 2), "{counts:?}");
        assert!(counts.contains(&0) && counts.contains(&2));
        for breath in &particles.particles {
            let offset = breath.position - feet;
            assert!(offset.x.hypot(offset.z) <= 0.2 + 1e-6 && offset.y == 0.0);
            assert_eq!(breath.velocity, DVec3::ZERO);
        }
        let mut particles = AmbientParticles::new(12);
        particles.breath_cloud(feet, 3.0, false);
        assert_eq!(particles.len(), 29);
        let tenth = f64::from(0.1_f32);
        for breath in &particles.particles {
            let offset = breath.position - feet;
            assert!(offset.x.hypot(offset.z) <= 3.0 + 1e-4 && offset.y == 0.0);
            assert!(breath.velocity.x.abs() <= 0.075 && breath.velocity.z.abs() <= 0.075);
            assert_eq!(breath.velocity.y, (f64::from(0.01_f32) - tenth) + tenth);
        }
        assert!(particles.particles.iter().any(|breath| (breath.position - feet).length() > 1.5));
    }

    #[test]
    fn the_fireball_burst_sprays_two_hundred_breath_particles_from_its_block() {
        let mut particles = AmbientParticles::new(13);
        particles.breath_burst(DVec3::new(10.7, 64.2, -3.4));
        assert_eq!(particles.len(), 200);
        let tenth = f64::from(0.1_f32);
        for breath in &particles.particles {
            assert!((breath.position.x - 10.0).abs() <= 0.4 + 1e-9, "{}", breath.position);
            assert!((breath.position.z + 4.0).abs() <= 0.4 + 1e-9, "{}", breath.position);
            assert!((breath.position.y - 64.3).abs() < 1e-12);
            let power = ((breath.position.x - 10.0).hypot(breath.position.z + 4.0) * 10.0) as f32;
            let reach = f64::from(power).max(1e-3);
            assert!(breath.velocity.x.hypot(breath.velocity.z) <= reach * reach + 1e-3, "{breath:?}");
            assert!(breath.velocity.y >= (0.01 - tenth) * reach + tenth - 1e-3);
            assert!(breath.velocity.y <= (0.51 - tenth) * reach + tenth + 1e-3);
        }
        assert!(particles.particles.iter().any(|breath| breath.velocity.x.hypot(breath.velocity.z) > 4.0));
        let mut trail = AmbientParticles::new(14);
        trail.breath_trail(DVec3::new(1.0, 80.0, 2.0));
        assert_eq!(trail.particles[0].position, DVec3::new(1.0, 80.5, 2.0));
        assert_eq!(trail.particles[0].velocity, DVec3::ZERO);
    }

    fn breath_of(kind: crate::ender_dragon::BreathKind, motion: DVec3) -> crate::ender_dragon::DragonBreath {
        crate::ender_dragon::DragonBreath {
            head: DVec3::new(3.0, 70.5, -2.0),
            look: DVec3::new(0.6, 0.8, 0.0),
            motion,
            kind,
        }
    }

    #[test]
    fn landing_breath_streams_eight_particles_back_along_the_look() {
        let motion = DVec3::new(0.1, -0.2, 0.05);
        let breath = breath_of(crate::ender_dragon::BreathKind::Landing, motion);
        let mut particles = AmbientParticles::new(21);
        particles.mouth_breath(&breath);
        assert_eq!(particles.len(), 8);
        let mut random = LegacyRandom::new(21);
        let tenth = f64::from(0.1_f32);
        let expected_motion = DVec3::new(
            -0.6 * f64::from(0.08_f32) + 0.1,
            (-0.8 * f64::from(0.3_f32) - 0.2 - tenth) + tenth,
            0.05,
        );
        for particle in &particles.particles {
            let x = 3.0 + random.next_gaussian() / 2.0;
            let y = 70.5 + random.next_gaussian() / 2.0;
            let z = -2.0 + random.next_gaussian() / 2.0;
            for _ in 0..5 {
                random.next_float();
            }
            assert_eq!(particle.position, DVec3::new(x, y, z));
            assert!((particle.velocity - expected_motion).length() < 1e-12, "{:?}", particle.velocity);
            assert_eq!(particle.kind, Kind::DragonBreath);
        }
    }

    #[test]
    fn flame_breath_fans_six_speeds_at_eight_points() {
        let breath = breath_of(crate::ender_dragon::BreathKind::Flame, DVec3::new(5.0, 5.0, 5.0));
        let mut particles = AmbientParticles::new(22);
        particles.mouth_breath(&breath);
        assert_eq!(particles.len(), 48);
        let tenth = f64::from(0.1_f32);
        for (point, fan) in particles.particles.chunks(6).enumerate() {
            for (speed, particle) in fan.iter().enumerate() {
                assert_eq!(particle.position, fan[0].position, "point {point}");
                let expected = DVec3::new(
                    -0.6 * f64::from(0.08_f32) * speed as f64,
                    (-0.8 * f64::from(0.6_f32) - tenth) + tenth,
                    0.0,
                );
                assert!((particle.velocity - expected).length() < 1e-12, "point {point} speed {speed}: {:?}", particle.velocity);
            }
            assert!((fan[0].position - breath.head).length() < 4.0);
        }
        let points: Vec<DVec3> = particles.particles.chunks(6).map(|fan| fan[0].position).collect();
        assert!(points.windows(2).all(|pair| pair[0] != pair[1]));
    }

    #[test]
    fn a_totem_emitter_runs_thirty_ticks_inside_the_body() {
        let mut particles = AmbientParticles::new(41);
        let body = Body {
            feet: DVec3::new(10.5, 64.0, -3.5),
            width: 0.6,
            height: 1.8,
        };
        particles.totem_emitter(7, body);
        let mut emissions = 1;
        let mut counts = vec![particles.len()];
        while !particles.emitters.is_empty() {
            let before = particles.len();
            particles.tick_emitters(|key| (key == 7).then_some(body));
            counts.push(particles.len() - before);
            emissions += 1;
        }
        assert_eq!(emissions, TOTEM_EMITTER_TICKS);
        assert!(counts.iter().all(|count| (1..=TOTEM_TRIES).contains(count)), "{counts:?}");
        let before = particles.len();
        particles.tick_emitters(|_| Some(body));
        assert_eq!(particles.len(), before);
        for particle in &particles.particles {
            assert_eq!(particle.kind, Kind::Totem);
            let offset = particle.position - body.feet;
            assert!(offset.x.abs() <= 0.15 && offset.z.abs() <= 0.15, "{offset}");
            assert!((0.45..=1.35).contains(&offset.y), "{offset}");
            let unit = DVec3::new(
                offset.x / 0.15,
                (offset.y / 1.8 - 0.5) * 4.0,
                offset.z / 0.15,
            );
            assert!(unit.length() <= 1.0 + 1e-6);
            let motion = DVec3::new(unit.x, unit.y + 0.2, unit.z);
            assert!((particle.velocity - motion).length() < 1e-6);
        }
        let mut gone = AmbientParticles::new(43);
        gone.totem_emitter(9, body);
        let first = gone.len();
        gone.tick_emitters(|_| None);
        assert!(gone.emitters.is_empty());
        assert_eq!(gone.len(), first);
    }

    #[test]
    fn totem_glitter_is_mostly_green_and_fades_after_half_life() {
        let mut particles = AmbientParticles::new(47);
        for _ in 0..2000 {
            particles.add(Kind::Totem, DVec3::new(0.5, 80.0, 0.5), DVec3::ZERO);
        }
        let yellow = particles
            .particles
            .iter()
            .filter(|particle| particle.color[0] >= 0.6)
            .count();
        assert!((400..600).contains(&yellow), "{yellow}");
        for particle in &particles.particles {
            let [r, g, b] = particle.color;
            if r >= 0.6 {
                assert!(r <= 0.8 && (0.6..=0.9).contains(&g) && b <= 0.2);
            } else {
                assert!((0.1..=0.3).contains(&r) && (0.4..=0.7).contains(&g) && b <= 0.2);
            }
            assert!((60..72).contains(&particle.lifetime));
            assert!(particle.quad_size >= 0.075 - 1e-6 && particle.quad_size <= 0.15 + 1e-6);
        }
        assert!(particles_translucent(Kind::Totem));
        let mut one = AmbientParticles::new(53);
        one.add(Kind::Totem, DVec3::new(0.5, 80.0, 0.5), DVec3::new(0.0, 0.5, 0.0));
        let lifetime = one.particles[0].lifetime;
        assert_eq!(
            one.particles[0].sprite().map(|id| id.key()).as_deref(),
            Some("minecraft:particle/glitter_7")
        );
        let scene = open();
        one.tick(&scene);
        let ticked = &one.particles[0];
        let rise = (0.5 - 0.04 * f64::from(1.25_f32)) * f64::from(0.6_f32);
        assert!((ticked.velocity.y - rise).abs() < 1e-9, "{}", ticked.velocity.y);
        assert_eq!(ticked.alpha, 1.0);
        while one.particles[0].age < lifetime / 2 {
            one.tick(&scene);
            assert_eq!(one.particles[0].alpha, 1.0);
        }
        one.tick(&scene);
        let late = &one.particles[0];
        let expected = 1.0 - (late.age - lifetime / 2) as f32 / lifetime as f32;
        assert!((late.alpha - expected).abs() < 1e-6);
        assert!(late.alpha < 1.0);
        for _ in 0..lifetime {
            one.tick(&scene);
        }
        assert!(one.is_empty());
    }

    fn particles_translucent(kind: Kind) -> bool {
        let mut particles = AmbientParticles::new(1);
        particles.add(kind, DVec3::ZERO, DVec3::ZERO);
        particles.particles[0].translucent()
    }

    #[test]
    fn trial_detection_particles_rise_lit_through_their_frames_and_the_trial_omen_drifts() {
        let start = DVec3::new(0.5, 64.5, 0.5);
        for kind in [Kind::TrialSpawnerDetection, Kind::TrialSpawnerDetectionOminous] {
            for seed in 0..50 {
                let mut particles = AmbientParticles::new(seed);
                particles.add(kind, start, DVec3::ZERO);
                let particle = particles.particles[0].clone();
                assert!((12..=24).contains(&particle.lifetime), "{}", particle.lifetime);
                assert_eq!((particle.velocity.x, particle.velocity.z), (0.0, 0.0));
                assert!(!particle.translucent());
                assert_eq!(particle.size_at(0.0), 0.0);
                let mut frames = Vec::new();
                while !particles.is_empty() {
                    let now = &particles.particles[0];
                    frames.push(now.sprite().unwrap().key());
                    particles.tick(&open());
                }
                let sheet = if kind == Kind::TrialSpawnerDetection {
                    "minecraft:particle/trial_spawner_detection"
                } else {
                    "minecraft:particle/trial_spawner_detection_ominous"
                };
                assert_eq!(frames[0], format!("{sheet}_0"));
                assert_eq!(frames.last(), Some(&format!("{sheet}_4")));
            }
        }
        let light = SkyLight::from_levels((0, 64, 0), (1, 1, 1), vec![12], vec![3]);
        let mut particles = AmbientParticles::new(1);
        particles.add(Kind::TrialSpawnerDetection, start, DVec3::ZERO);
        particles.tick(&open());
        let mut opaque = ChunkMesh::default();
        let mut translucent = ChunkMesh::default();
        let atlas = crate::mesh::make_atlas(&crate::pack::PackStack::open(Vec::new()).unwrap(), textures()).unwrap();
        particles.append_mesh(&mut opaque, &mut translucent, &atlas, Vec3::Z, 0.0, &light);
        assert_eq!(opaque.vertices.len(), 4);
        assert!(opaque.vertices.iter().all(|v| v.block_light == 15.0), "full block light");
        let mut particles = AmbientParticles::new(3);
        particles.add(Kind::TrialOmen, start, DVec3::new(0.01, 0.0, -0.01));
        let omen = particles.particles[0].clone();
        assert!(omen.translucent());
        assert_eq!(omen.sprite().unwrap().key(), "minecraft:particle/trial_omen");
        assert!((8..=40).contains(&omen.lifetime), "{}", omen.lifetime);
        assert_eq!(omen.size_at(0.5), omen.quad_size);
        for name in ["trial_omen", "trial_spawner_detection_4", "trial_spawner_detection_ominous_0"] {
            assert!(textures().contains(&ResourceId::parse(&format!("minecraft:particle/{name}")).unwrap()));
        }
    }

    #[test]
    fn ominous_spawning_particles_fly_into_their_item_and_turn_white() {
        let home = DVec3::new(0.5, 70.0, 0.5);
        for seed in 0..40 {
            let mut particles = AmbientParticles::new(seed);
            particles.ominous_spawning(home);
            assert!((1..=3).contains(&particles.particles.len()));
            for particle in &particles.particles {
                assert_eq!(particle.kind, Kind::OminousSpawning);
                assert!((25..=29).contains(&particle.lifetime), "{}", particle.lifetime);
                assert_eq!(particle.position, home + particle.velocity);
                assert!(!particle.translucent());
                assert_eq!(particle.size_at(0.5), particle.quad_size);
                assert!(particle.quad_size >= 0.1 * 0.2 * 3.0 && particle.quad_size <= 0.1 * 0.7 * 5.0);
            }
            let start = particles.particles[0].position;
            let mut last = particles.particles[0].clone();
            while !particles.is_empty() {
                last = particles.particles[0].clone();
                particles.tick(&open());
            }
            assert!(last.position.distance(home) < start.distance(home) * 0.1 + 1.0e-9);
            assert!(last.color.iter().all(|channel| *channel > 0.9), "{:?}", last.color);
        }
        let particle = Particle::ominous_spawning(DVec3::ZERO, DVec3::X, &mut LegacyRandom::new(1));
        assert_eq!(particle.sprite().unwrap().key(), "minecraft:particle/ominous_spawning");
        assert_eq!(particle.color, OMINOUS_SPAWNING_START);
        assert!(textures().contains(&ResourceId::parse("minecraft:particle/ominous_spawning").unwrap()));
    }

    #[test]
    fn vault_connection_particles_fly_from_their_player_into_the_vault_and_fade_in() {
        let keyhole = DVec3::new(4.5, 65.75, 5.0);
        let toward = DVec3::new(2.0, -1.0, 3.0);
        for seed in 0..20 {
            let mut particles = AmbientParticles::new(seed);
            particles.add(Kind::VaultConnection, keyhole, toward);
            let particle = particles.particles[0].clone();
            assert_eq!(particle.position, keyhole + toward, "it starts at the player");
            assert!((30..=39).contains(&particle.lifetime), "{}", particle.lifetime);
            assert!(particle.translucent() && particle.alpha == 0.0);
            assert!(particle.quad_size >= 0.1 * 0.2 * 1.5 && particle.quad_size <= 0.1 * 0.7 * 1.5);
            assert!(particle.color[2] >= 0.4 && particle.color[0] == particle.color[2] * 0.9);
            let mut last = particle.clone();
            while !particles.is_empty() {
                last = particles.particles[0].clone();
                particles.tick(&open());
            }
            assert!((last.position - (keyhole - DVec3::new(0.0, 1.2, 0.0))).length() < 1.0e-5, "it dips into the vault: {:?}", last.position);
            assert!((last.alpha - 0.6).abs() < 1.0e-6);
        }
        let particle = Particle::vault_connection(DVec3::ZERO, DVec3::X, &mut LegacyRandom::new(1));
        assert_eq!(particle.sprite().unwrap().key(), "minecraft:particle/vault_connection");
        assert!(textures().contains(&ResourceId::parse("minecraft:particle/vault_connection").unwrap()));
    }
}
