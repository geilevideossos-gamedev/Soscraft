use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::rng::LegacyRandom;

const SPRITES: i32 = 8;
const FRICTION: f64 = 0.96;
const GRAVITY: f64 = -0.1;
const CAP: usize = 16384;
const BURST: usize = 100;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpellKind {
    Effect,
    Instant,
}

impl SpellKind {
    fn sheet(self) -> &'static str {
        match self {
            Self::Effect => "effect",
            Self::Instant => "spell",
        }
    }
}

#[derive(Clone, Debug)]
struct Spell {
    kind: SpellKind,
    position: DVec3,
    previous: DVec3,
    velocity: DVec3,
    color: [f32; 3],
    size: f32,
    age: i32,
    lifetime: i32,
}

pub struct SpellParticles {
    particles: Vec<Spell>,
    random: LegacyRandom,
}

pub fn break_center(at: DVec3) -> DVec3 {
    DVec3::new(at.x.floor() + 0.5, at.y.floor(), at.z.floor() + 0.5)
}

pub fn rgb(color: u32) -> [f32; 3] {
    [
        ((color >> 16) & 255) as f32 / 255.0,
        ((color >> 8) & 255) as f32 / 255.0,
        (color & 255) as f32 / 255.0,
    ]
}

pub fn sprite(kind: SpellKind, age: i32, lifetime: i32) -> Option<ResourceId> {
    let step = if lifetime <= 0 {
        0
    } else {
        (age * (SPRITES - 1) / lifetime).clamp(0, SPRITES - 1)
    };
    ResourceId::parse(&format!(
        "minecraft:particle/{}_{}",
        kind.sheet(),
        SPRITES - 1 - step
    ))
    .ok()
}

pub fn textures() -> Vec<ResourceId> {
    [SpellKind::Effect, SpellKind::Instant]
        .into_iter()
        .flat_map(|kind| {
            (0..SPRITES).filter_map(move |index| {
                ResourceId::parse(&format!("minecraft:particle/{}_{index}", kind.sheet())).ok()
            })
        })
        .collect()
}

impl SpellParticles {
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

    pub fn burst(&mut self, at: DVec3, color: u32, instant: bool) {
        let center = break_center(at);
        let [r, g, b] = rgb(color);
        let kind = if instant {
            SpellKind::Instant
        } else {
            SpellKind::Effect
        };
        for _ in 0..BURST {
            let power = self.random.next_double() * 4.0;
            let angle = self.random.next_double() * std::f64::consts::PI * 2.0;
            let dx = angle.cos() * power;
            let dy = 0.01 + self.random.next_double() * 0.5;
            let dz = angle.sin() * power;
            let shade = 0.75 + self.random.next_float() * 0.25;
            let at = DVec3::new(center.x + dx * 0.1, center.y + 0.3, center.z + dz * 0.1);
            self.spawn(
                kind,
                at,
                DVec3::new(dx, dy, dz),
                [r * shade, g * shade, b * shade],
                power as f32,
            );
        }
    }

    pub fn cloud(&mut self, center: DVec3, radius: f32, color: u32, waiting: bool) {
        if waiting && self.random.next_boolean() {
            return;
        }
        let (count, spread) = if waiting {
            (2, 0.2)
        } else {
            (
                (std::f32::consts::PI * radius * radius).ceil() as i32,
                radius,
            )
        };
        for _ in 0..count {
            let angle = self.random.next_float() * std::f32::consts::TAU;
            let distance = self.random.next_float().sqrt() * spread;
            let at = center
                + DVec3::new(
                    f64::from(angle.cos() * distance),
                    0.0,
                    f64::from(angle.sin() * distance),
                );
            let color = if waiting && self.random.next_boolean() {
                [1.0; 3]
            } else {
                rgb(color)
            };
            self.spawn(SpellKind::Effect, at, DVec3::ZERO, color, 1.0);
        }
    }

    pub fn spawn(
        &mut self,
        kind: SpellKind,
        at: DVec3,
        motion: DVec3,
        color: [f32; 3],
        power: f32,
    ) {
        let xa = 0.5 - self.random.next_double();
        let za = 0.5 - self.random.next_double();
        let mut velocity = DVec3::new(
            xa + (self.random.next_double() * 2.0 - 1.0) * f64::from(0.4_f32),
            motion.y + (self.random.next_double() * 2.0 - 1.0) * f64::from(0.4_f32),
            za + (self.random.next_double() * 2.0 - 1.0) * f64::from(0.4_f32),
        );
        let speed =
            (self.random.next_double() + self.random.next_double() + 1.0) * f64::from(0.15_f32);
        let length = velocity.length();
        if length > 0.0 {
            velocity = velocity / length * speed * f64::from(0.4_f32);
        }
        velocity.y += f64::from(0.1_f32);
        velocity.y *= f64::from(0.2_f32);
        if motion.x == 0.0 && motion.z == 0.0 {
            velocity.x *= f64::from(0.1_f32);
            velocity.z *= f64::from(0.1_f32);
        }
        let size = 0.1 * (self.random.next_float() * 0.5 + 0.5) * 2.0 * 0.75;
        let lifetime = (8.0 / (f64::from(self.random.next_float()) * 0.8 + 0.2)) as i32;
        let power = f64::from(power);
        velocity.x *= power;
        velocity.y = (velocity.y - 0.1) * power + 0.1;
        velocity.z *= power;
        self.particles.push(Spell {
            kind,
            position: at,
            previous: at,
            velocity,
            color,
            size,
            age: 0,
            lifetime,
        });
        if self.particles.len() > CAP {
            self.particles.remove(0);
        }
    }

    pub fn tick(&mut self) {
        for spell in &mut self.particles {
            spell.previous = spell.position;
            spell.age += 1;
            spell.velocity.y -= 0.04 * GRAVITY;
            spell.position += spell.velocity;
            spell.velocity *= FRICTION;
        }
        self.particles.retain(|spell| spell.age <= spell.lifetime);
    }

    pub fn append_mesh(
        &self,
        mesh: &mut ChunkMesh,
        atlas: &Atlas,
        forward: Vec3,
        partial: f32,
        light: &SkyLight,
    ) {
        let right = forward.cross(Vec3::Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        for spell in &self.particles {
            let Some(id) = sprite(spell.kind, spell.age, spell.lifetime) else {
                continue;
            };
            if !atlas.contains(&id) {
                continue;
            }
            let [u0, v0, u1, v1] = atlas.region(&id);
            let center = spell
                .previous
                .lerp(spell.position, f64::from(partial.clamp(0.0, 1.0)))
                .as_vec3();
            let cell = (
                center.x.floor() as i32,
                center.y.floor() as i32,
                center.z.floor() as i32,
            );
            let (sky, block) = (light.get(cell) as f32, light.get_block(cell) as f32);
            let color = [spell.color[0], spell.color[1], spell.color[2], 1.0];
            let start = mesh.vertices.len() as u32;
            for (corner, uv) in [
                (Vec2::new(-1.0, -1.0), [u0, v1]),
                (Vec2::new(-1.0, 1.0), [u0, v0]),
                (Vec2::new(1.0, 1.0), [u1, v0]),
                (Vec2::new(1.0, -1.0), [u1, v1]),
            ] {
                let point = center + (right * corner.x + up * corner.y) * spell.size;
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

    #[cfg(test)]
    fn colors(&self) -> Vec<[f32; 3]> {
        self.particles.iter().map(|spell| spell.color).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_break_swirls_a_hundred_spells_of_its_colour_from_the_block_bottom() {
        let mut spells = SpellParticles::new(1);
        spells.burst(DVec3::new(3.7, 64.2, -1.3), 0x385DC6, false);
        assert_eq!(spells.len(), 100);
        let [r, g, b] = rgb(0x385DC6);
        for color in spells.colors() {
            let shade = color[0] / r;
            assert!((0.75..=1.0).contains(&shade), "{shade}");
            assert!((color[1] - g * shade).abs() < 1e-5 && (color[2] - b * shade).abs() < 1e-5);
        }
        let center = break_center(DVec3::new(3.7, 64.2, -1.3));
        assert_eq!(center, DVec3::new(3.5, 64.0, -1.5));
        assert!(
            spells
                .particles
                .iter()
                .all(|s| (s.position.y - 64.3).abs() < 1e-9
                    && (s.position - center).length() < 0.6
                    && s.kind == SpellKind::Effect)
        );
    }

    #[test]
    fn spells_drift_up_and_are_gone_within_forty_ticks() {
        let mut spells = SpellParticles::new(2);
        spells.burst(DVec3::new(0.5, 70.0, 0.5), 0xF82423, true);
        let lowest = spells.particles.iter().map(|s| s.lifetime).min().unwrap();
        assert!(lowest >= 8);
        assert!(spells.particles.iter().all(|s| s.lifetime <= 40));
        for _ in 0..41 {
            spells.tick();
        }
        assert!(spells.is_empty());
    }

    #[test]
    fn a_cloud_fills_its_circle_each_tick() {
        let mut spells = SpellParticles::new(3);
        let center = DVec3::new(0.5, 64.0, 0.5);
        spells.cloud(center, 3.0, 8889187, false);
        assert_eq!(spells.len(), 29);
        assert!(spells.particles.iter().all(|s| {
            let d = s.position - center;
            d.y == 0.0 && d.x * d.x + d.z * d.z <= 9.0 + 1e-6
        }));
        let mut waiting = SpellParticles::new(4);
        for _ in 0..40 {
            waiting.cloud(center, 3.0, 8889187, true);
        }
        assert!(
            waiting.len() < 80 && waiting.len() > 20,
            "{}",
            waiting.len()
        );
    }

    #[test]
    fn the_sprite_shrinks_from_the_seventh_to_the_first_frame() {
        let first = sprite(SpellKind::Effect, 0, 20).map(|id| id.key());
        let last = sprite(SpellKind::Instant, 20, 20).map(|id| id.key());
        assert_eq!(first.as_deref(), Some("minecraft:particle/effect_7"));
        assert_eq!(last.as_deref(), Some("minecraft:particle/spell_0"));
        assert_eq!(textures().len(), 16);
    }
}
