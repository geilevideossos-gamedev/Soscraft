use crate::lighting::SkyLight;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;
use glam::{DVec3, Quat, Vec2, Vec3};

pub const SPRITE: &str = "minecraft:particle/vibration";
const QUAD_SIZE: f32 = 0.3;
const FULL_BRIGHT: f32 = 15.0;
const WOBBLE_SPEED: f32 = 0.05;
const WOBBLE_SCALE: f32 = 2.0;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct VibrationView {
    pub from: DVec3,
    pub to: DVec3,
    pub ticks: i32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Signal {
    position: DVec3,
    previous: DVec3,
    target: DVec3,
    age: i32,
    lifetime: i32,
    rot: f32,
    rot_o: f32,
    pitch: f32,
    pitch_o: f32,
}

fn facing(position: DVec3, target: DVec3) -> (f32, f32) {
    let d = position - target;
    let rot = d.x.atan2(d.z) as f32;
    let pitch = d.y.atan2(d.x.hypot(d.z)) as f32;
    (rot, pitch)
}

impl Signal {
    fn new(view: VibrationView) -> Self {
        let (rot, pitch) = facing(view.from, view.to);
        Self {
            position: view.from,
            previous: view.from,
            target: view.to,
            age: 0,
            lifetime: view.ticks.max(0),
            rot,
            rot_o: rot,
            pitch,
            pitch_o: pitch,
        }
    }

    fn tick(&mut self) -> bool {
        self.previous = self.position;
        let prior = self.age;
        self.age += 1;
        if prior >= self.lifetime {
            return false;
        }
        let remaining = self.lifetime - self.age;
        if remaining <= 0 {
            return false;
        }
        let share = 1.0 / f64::from(remaining);
        self.position = self.position.lerp(self.target, share);
        self.rot_o = self.rot;
        self.pitch_o = self.pitch;
        (self.rot, self.pitch) = facing(self.position, self.target);
        true
    }

    fn turns(&self, partial: f32) -> [Quat; 2] {
        let wobble = ((self.age as f32 + partial - std::f32::consts::TAU) * WOBBLE_SPEED).sin()
            * WOBBLE_SCALE;
        let rot = self.rot_o + partial * (self.rot - self.rot_o);
        let pitch =
            self.pitch_o + partial * (self.pitch - self.pitch_o) + std::f32::consts::FRAC_PI_2;
        [
            Quat::from_rotation_y(rot)
                * Quat::from_rotation_x(-pitch)
                * Quat::from_rotation_y(wobble),
            Quat::from_rotation_y(rot - std::f32::consts::PI)
                * Quat::from_rotation_x(pitch)
                * Quat::from_rotation_y(wobble),
        ]
    }
}

#[derive(Clone, Debug, Default)]
pub struct VibrationSignals {
    signals: Vec<Signal>,
}

impl VibrationSignals {
    pub fn add(&mut self, view: VibrationView) {
        self.signals.push(Signal::new(view));
    }

    pub fn tick(&mut self) {
        self.signals.retain_mut(Signal::tick);
    }

    pub fn len(&self) -> usize {
        self.signals.len()
    }

    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
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
        for signal in &self.signals {
            let centre = signal.previous.lerp(signal.position, f64::from(partial));
            let cell = (
                centre.x.floor() as i32,
                centre.y.floor() as i32,
                centre.z.floor() as i32,
            );
            let sky = f32::from(light.get(cell));
            let centre = centre.as_vec3();
            for turn in signal.turns(partial) {
                let start = translucent.vertices.len() as u32;
                for (corner, uv) in [
                    (Vec2::new(-1.0, -1.0), [u1, v1]),
                    (Vec2::new(-1.0, 1.0), [u1, v0]),
                    (Vec2::new(1.0, 1.0), [u0, v0]),
                    (Vec2::new(1.0, -1.0), [u0, v1]),
                ] {
                    let point = centre + turn * Vec3::new(corner.x, corner.y, 0.0) * QUAD_SIZE;
                    translucent.vertices.push(Vertex {
                        position: point.to_array(),
                        uv,
                        color: [1.0; 4],
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
    fn a_ripple_reaches_its_listener_on_its_last_tick_and_goes() {
        let mut signals = VibrationSignals::default();
        let from = DVec3::new(13.5, 64.0, 8.5);
        let to = DVec3::new(8.5, 64.5, 8.5);
        signals.add(VibrationView { from, to, ticks: 5 });
        let mut path = Vec::new();
        while !signals.is_empty() {
            signals.tick();
            path.extend(signals.signals.first().map(|signal| signal.position));
        }
        assert_eq!(path.len(), 4);
        assert!(path[3].distance(to) < 1e-9, "{path:?}");
        assert!(
            path.windows(2)
                .all(|pair| pair[1].distance(to) < pair[0].distance(to))
        );
        let mut instant = VibrationSignals::default();
        instant.add(VibrationView {
            from: to,
            to,
            ticks: 0,
        });
        instant.tick();
        assert!(instant.is_empty());
    }

    #[test]
    fn a_ripple_turns_along_its_path() {
        let signal = Signal::new(VibrationView {
            from: DVec3::new(0.5, 64.5, 5.5),
            to: DVec3::new(0.5, 64.5, 0.5),
            ticks: 5,
        });
        assert!(signal.rot.abs() < 1e-6 && signal.pitch.abs() < 1e-6);
        let [front, back] = signal.turns(0.0);
        let normal = |turn: Quat| turn * Vec3::Z;
        assert!(
            normal(front).dot(normal(back)) < -0.99,
            "two faces back to back"
        );
        let sideways = Signal::new(VibrationView {
            from: DVec3::new(5.5, 64.5, 0.5),
            to: DVec3::new(0.5, 64.5, 0.5),
            ticks: 5,
        });
        assert!((sideways.rot - std::f32::consts::FRAC_PI_2).abs() < 1e-6);
        assert_eq!(textures().len(), 1);
    }
}
