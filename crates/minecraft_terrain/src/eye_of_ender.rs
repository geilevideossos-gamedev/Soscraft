use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use glam::{DVec3, Vec2, Vec3};
use minecraftoss_player::rng::LegacyRandom;
use minecraftoss_world::chunk_map::WorldGen;
use minecraftoss_world::level::Level;
use minecraftoss_world::level::burning::LevelSound;

use crate::ender_eye::{ENDER_EYE, EyeThrow};
use crate::locate::Search;
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::portal_particles::PortalParticles;

pub const LAUNCH_SOUND: &str = "minecraft:entity.ender_eye.launch";
pub const DEATH_SOUND: &str = "minecraft:entity.ender_eye.death";
pub const LOCATED_TAG: &str = "minecraft:eye_of_ender_located";
pub const SEARCH_RADIUS: i32 = 100;
pub const SEARCH_CAP: Duration = Duration::from_secs(20);
pub const LIFE: i32 = 80;
pub const TOO_FAR: f64 = 12.0;
pub const TOO_FAR_HEIGHT: f64 = 8.0;
const FULL_BRIGHT: f32 = 15.0;

pub type Crumbs = (String, DVec3, Vec<Vec3>);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeEnd {
    Drop,
    Shatter,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EyeOfEnder {
    pub owner: u64,
    pub components: Option<String>,
    pub position: DVec3,
    pub previous: DVec3,
    pub motion: DVec3,
    pub target: DVec3,
    pub life: i32,
    pub age: i32,
    pub survives: bool,
}

pub fn launch_pitch(random: f32) -> f32 {
    0.33 + random * (0.5 - 0.33)
}

pub fn signal_target(from: DVec3, to: DVec3) -> DVec3 {
    let delta = to - from;
    let distance = (delta.x * delta.x + delta.z * delta.z).sqrt();
    if distance > TOO_FAR {
        from + DVec3::new(
            delta.x / distance * TOO_FAR,
            TOO_FAR_HEIGHT,
            delta.z / distance * TOO_FAR,
        )
    } else {
        to
    }
}

pub fn steer(old: DVec3, at: DVec3, target: DVec3) -> DVec3 {
    let toward = DVec3::new(target.x - at.x, 0.0, target.z - at.z);
    let distance = toward.length();
    let old_speed = (old.x * old.x + old.z * old.z).sqrt();
    let mut speed = old_speed + 0.0025 * (distance - old_speed);
    let mut rise = old.y;
    if distance < 1.0 {
        speed *= 0.8;
        rise *= 0.8;
    }
    let up = if at.y - old.y < target.y { 1.0 } else { -1.0 };
    let horizontal = if distance > 0.0 {
        toward * (speed / distance)
    } else {
        DVec3::ZERO
    };
    horizontal + DVec3::new(0.0, rise + (up - rise) * 0.015, 0.0)
}

impl EyeOfEnder {
    pub fn launch(owner: u64, throw: &EyeThrow, found: DVec3, random: &mut LegacyRandom) -> Self {
        let mut eye = Self {
            owner,
            components: throw.components.clone(),
            position: throw.middle,
            previous: throw.middle,
            motion: DVec3::ZERO,
            target: throw.middle,
            life: 0,
            age: 0,
            survives: false,
        };
        eye.signal_to(found, random);
        eye
    }

    pub fn signal_to(&mut self, to: DVec3, random: &mut LegacyRandom) {
        self.target = signal_target(self.position, to);
        self.life = 0;
        self.survives = random.next_int(5) > 0;
    }

    pub fn tick(&mut self) -> Option<EyeEnd> {
        self.previous = self.position;
        self.age += 1;
        let next = self.position + self.motion;
        self.motion = steer(self.motion, next, self.target);
        self.position = next;
        self.life += 1;
        if self.life > LIFE {
            Some(if self.survives {
                EyeEnd::Drop
            } else {
                EyeEnd::Shatter
            })
        } else {
            None
        }
    }

    pub fn view(&self) -> EyeView {
        EyeView {
            owner: self.owner,
            previous: self.previous,
            position: self.position,
            motion: self.motion,
            age: self.age,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeView {
    pub owner: u64,
    pub previous: DVec3,
    pub position: DVec3,
    pub motion: DVec3,
    pub age: i32,
}

impl EyeView {
    pub fn at(&self, partial: f32) -> DVec3 {
        self.previous
            .lerp(self.position, f64::from(partial.clamp(0.0, 1.0)))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EyeSnapshot {
    pub views: Vec<EyeView>,
    pub shattered: Vec<[i32; 3]>,
}

pub fn located_structures(gen: &WorldGen) -> Vec<String> {
    crate::locate::tags(gen, "worldgen/structure")
        .remove(LOCATED_TAG)
        .into_iter()
        .flatten()
        .collect()
}

pub fn block_of(at: DVec3) -> [i32; 3] {
    [
        at.x.floor() as i32,
        at.y.floor() as i32,
        at.z.floor() as i32,
    ]
}

pub struct Eyes {
    flying: Vec<EyeOfEnder>,
    shattered: Vec<[i32; 3]>,
    targets: Option<Vec<String>>,
    random: LegacyRandom,
}

impl Default for Eyes {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self::seeded(seed ^ 0xE7E0_F0E4)
    }
}

impl Eyes {
    pub fn seeded(seed: u64) -> Self {
        Self {
            flying: Vec::new(),
            shattered: Vec::new(),
            targets: None,
            random: LegacyRandom::new(seed),
        }
    }

    pub fn flying(&self) -> &[EyeOfEnder] {
        &self.flying
    }

    pub fn locate(&mut self, gen: &WorldGen, feet: DVec3) -> Option<DVec3> {
        let targets = self.targets.get_or_insert_with(|| located_structures(gen));
        let deadline = Instant::now() + SEARCH_CAP;
        match crate::locate::nearest_structure(
            gen,
            targets,
            block_of(feet),
            SEARCH_RADIUS,
            deadline,
        ) {
            Search::Found([x, y, z], _) => {
                Some(DVec3::new(f64::from(x), f64::from(y), f64::from(z)))
            }
            Search::NotFound | Search::TimedOut => None,
        }
    }

    pub fn launch(&mut self, level: &mut Level, owner: u64, throw: &EyeThrow, found: DVec3) {
        let eye = EyeOfEnder::launch(owner, throw, found, &mut self.random);
        self.flying.push(eye);
        let pitch = launch_pitch(self.random.next_float());
        level.sounds.push(LevelSound {
            event: LAUNCH_SOUND,
            pos: throw.feet.to_array(),
            volume: 1.0,
            pitch,
        });
    }

    pub fn tick(&mut self, level: &mut Level) {
        let mut ended = Vec::new();
        self.flying.retain_mut(|eye| match eye.tick() {
            Some(end) => {
                ended.push((eye.position, eye.components.clone(), end));
                false
            }
            None => true,
        });
        for (at, components, end) in ended {
            level.sounds.push(LevelSound {
                event: DEATH_SOUND,
                pos: at.to_array(),
                volume: 1.0,
                pitch: 1.0,
            });
            match end {
                EyeEnd::Drop => {
                    let mut stack = minecraftoss_core::item::ItemStack::new(ENDER_EYE, 1);
                    stack.components = components
                        .as_deref()
                        .and_then(crate::item_nbt::components_from_text);
                    let dx = self.random.next_double() * 0.2 - 0.1;
                    let dz = self.random.next_double() * 0.2 - 0.1;
                    level.spawn_item_with(at.to_array(), stack, [dx, 0.2, dz], 0, 0);
                }
                EyeEnd::Shatter => self.shattered.push(block_of(at)),
            }
        }
    }

    pub fn snapshot(&mut self) -> EyeSnapshot {
        EyeSnapshot {
            views: self.flying.iter().map(EyeOfEnder::view).collect(),
            shattered: std::mem::take(&mut self.shattered),
        }
    }
}

pub fn trail(view: &EyeView, random: &mut LegacyRandom) -> (DVec3, DVec3) {
    let at = view.position - view.motion * 0.25;
    let x = at.x + random.next_double() * 0.6 - 0.3;
    let z = at.z + random.next_double() * 0.6 - 0.3;
    (DVec3::new(x, at.y - 0.5, z), view.motion)
}

pub fn shatter_crumbs(block: [i32; 3], random: &mut LegacyRandom) -> Crumbs {
    let at = DVec3::new(
        f64::from(block[0]) + 0.5,
        f64::from(block[1]),
        f64::from(block[2]) + 0.5,
    );
    let velocities = (0..8)
        .map(|_| {
            let x = random.next_gaussian() * 0.15;
            let y = random.next_double() * 0.2;
            let z = random.next_gaussian() * 0.15;
            DVec3::new(x, y, z).as_vec3()
        })
        .collect();
    (ENDER_EYE.to_owned(), at, velocities)
}

pub fn shatter_ring(block: [i32; 3]) -> Vec<(DVec3, DVec3)> {
    let (x, y, z) = (
        f64::from(block[0]) + 0.5,
        f64::from(block[1]),
        f64::from(block[2]) + 0.5,
    );
    let mut out = Vec::new();
    let mut angle = 0.0_f64;
    while angle < 6.283185307179586 {
        let (sin, cos) = angle.sin_cos();
        let start = DVec3::new(x + cos * 5.0, y - 0.4, z + sin * 5.0);
        out.push((start, DVec3::new(cos * -5.0, 0.0, sin * -5.0)));
        out.push((start, DVec3::new(cos * -7.0, 0.0, sin * -7.0)));
        angle += 0.15707963267948966;
    }
    out
}

pub struct EyeTrails {
    views: Vec<EyeView>,
    random: LegacyRandom,
}

impl Default for EyeTrails {
    fn default() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self {
            views: Vec::new(),
            random: LegacyRandom::new(seed ^ 0x7A11_E7E5),
        }
    }
}

impl EyeTrails {
    pub fn views(&self) -> &[EyeView] {
        &self.views
    }

    pub fn receive(
        &mut self,
        snapshot: EyeSnapshot,
        portal: &mut PortalParticles,
        crumbs: &mut Vec<Crumbs>,
    ) {
        self.views = snapshot.views;
        for block in snapshot.shattered {
            crumbs.push(shatter_crumbs(block, &mut self.random));
            for (start, motion) in shatter_ring(block) {
                portal.emit(start, motion);
            }
        }
    }

    pub fn tick(&mut self, portal: &mut PortalParticles) {
        for view in &self.views {
            let (start, motion) = trail(view, &mut self.random);
            portal.emit(start, motion);
        }
    }
}

pub fn append_eyes(
    mesh: &mut ChunkMesh,
    views: &[EyeView],
    partial: f32,
    camera: DVec3,
    atlas: &Atlas,
    forward: Vec3,
) {
    let Some(texture) = crate::thrown::texture(ENDER_EYE).filter(|id| atlas.contains(id)) else {
        return;
    };
    let [u0, v0, u1, v1] = atlas.region(&texture);
    let right = forward.cross(Vec3::Y).normalize_or_zero();
    let up = right.cross(forward).normalize_or_zero();
    for view in views {
        let at = view.at(partial);
        if !crate::thrown::shown(view.age, at, camera) {
            continue;
        }
        let center = at.as_vec3() + up * 0.125;
        let start = mesh.vertices.len() as u32;
        for (corner, uv) in [
            (Vec2::new(-0.25, -0.25), [u0, v1]),
            (Vec2::new(0.25, -0.25), [u1, v1]),
            (Vec2::new(0.25, 0.25), [u1, v0]),
            (Vec2::new(-0.25, 0.25), [u0, v0]),
        ] {
            let point = center + right * corner.x + up * corner.y;
            mesh.vertices.push(Vertex {
                position: point.to_array(),
                uv,
                color: [1.0; 4],
                sky_light: FULL_BRIGHT,
                block_light: FULL_BRIGHT,
            });
        }
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraftoss_core::Registries;
    use minecraftoss_core::registries::DataPaths;
    use minecraftoss_generator::terrain::TerrainGenerator;
    use minecraftoss_world::level::entity::EntityKind;
    use std::sync::Arc;

    fn close(a: DVec3, b: DVec3) -> bool {
        (a - b).abs().max_element() < 1.0e-9
    }

    fn throw_at(middle: DVec3) -> EyeThrow {
        EyeThrow {
            feet: middle - DVec3::Y * 0.9,
            middle,
            components: None,
        }
    }

    #[test]
    fn a_far_target_aims_12_out_and_8_up() {
        let from = DVec3::new(0.5, 70.0, 0.5);
        assert!(close(
            signal_target(from, DVec3::new(1000.5, 0.0, 0.5)),
            DVec3::new(12.5, 78.0, 0.5)
        ));
        assert!(close(
            signal_target(from, DVec3::new(300.5, 0.0, 400.5)),
            DVec3::new(0.5 + 7.2, 78.0, 0.5 + 9.6)
        ));
        let near = DVec3::new(10.5, 20.0, 0.5);
        assert_eq!(signal_target(from, near), near);
        let just = DVec3::new(12.5, 0.0, 0.5);
        assert_eq!(signal_target(from, just), just, "exactly 12 is not too far");
    }

    #[test]
    fn the_first_ticks_match_hand_worked_vanilla_numbers() {
        let mut random = LegacyRandom::new(1);
        let middle = DVec3::new(0.5, 70.9, 0.5);
        let mut eye = EyeOfEnder::launch(
            3,
            &throw_at(middle),
            DVec3::new(1000.0, 0.0, 0.5),
            &mut random,
        );
        assert_eq!(eye.owner, 3);
        assert!(close(eye.target, DVec3::new(12.5, 78.9, 0.5)));
        assert_eq!(eye.tick(), None);
        assert!(close(eye.position, middle));
        assert!(close(eye.motion, DVec3::new(0.03, 0.015, 0.0)));
        assert_eq!(eye.tick(), None);
        assert!(close(eye.previous, middle));
        assert!(close(eye.position, middle + DVec3::new(0.03, 0.015, 0.0)));
        assert!(close(eye.motion, DVec3::new(0.05985, 0.029775, 0.0)));
    }

    #[test]
    fn steering_sinks_toward_a_lower_target_and_brakes_within_a_block() {
        let down = steer(
            DVec3::new(0.0, 0.1, 0.0),
            DVec3::new(0.0, 5.1, 0.0),
            DVec3::new(3.0, 0.0, 0.0),
        );
        assert!(close(down, DVec3::new(0.0075, 0.1 - 1.1 * 0.015, 0.0)));
        let near = steer(
            DVec3::new(0.5, 0.2, 0.0),
            DVec3::new(0.5, 0.2, 0.0),
            DVec3::new(1.0, 10.0, 0.0),
        );
        assert!(close(near, DVec3::new(0.4, 0.16 + 0.84 * 0.015, 0.0)));
        let above = steer(
            DVec3::ZERO,
            DVec3::new(2.0, 3.0, 2.0),
            DVec3::new(2.0, 9.0, 2.0),
        );
        assert!(close(above, DVec3::new(0.0, 0.015, 0.0)));
    }

    #[test]
    fn the_eye_ends_on_its_81st_tick_and_a_new_signal_restarts_its_life() {
        let mut random = LegacyRandom::new(5);
        let mut eye = EyeOfEnder::launch(
            0,
            &throw_at(DVec3::new(0.5, 64.9, 0.5)),
            DVec3::new(-500.0, 0.0, 200.0),
            &mut random,
        );
        for _ in 0..40 {
            assert_eq!(eye.tick(), None);
        }
        eye.signal_to(DVec3::new(-500.0, 0.0, 200.0), &mut random);
        for _ in 0..LIFE {
            assert_eq!(eye.tick(), None);
        }
        let end = eye.tick().expect("the 81st tick ends it");
        assert_eq!(end == EyeEnd::Drop, eye.survives);
        assert_eq!(eye.age, 121);
    }

    #[test]
    fn four_eyes_in_five_drop() {
        let throw = throw_at(DVec3::new(0.5, 64.9, 0.5));
        let mut drops = 0;
        for seed in 0..10_000u64 {
            let mut random = LegacyRandom::new(seed);
            let eye = EyeOfEnder::launch(0, &throw, DVec3::new(900.0, 0.0, 0.0), &mut random);
            assert_eq!(eye.survives, LegacyRandom::new(seed).next_int(5) > 0);
            drops += usize::from(eye.survives);
        }
        assert!((7_800..=8_200).contains(&drops), "{drops} of 10000");
    }

    #[test]
    fn the_launch_pitch_spans_a_third_to_a_half() {
        assert!((launch_pitch(0.0) - 0.33).abs() < 1.0e-6);
        assert!((launch_pitch(1.0) - 0.5).abs() < 1.0e-6);
        assert!((launch_pitch(0.5) - 0.415).abs() < 1.0e-6);
    }

    #[test]
    fn a_shattered_eye_leaves_8_crumbs_and_a_ring_of_80_portal_particles() {
        let mut random = LegacyRandom::new(2);
        let (item, at, velocities) = shatter_crumbs([10, 64, -3], &mut random);
        assert_eq!(item, ENDER_EYE);
        assert_eq!(at, DVec3::new(10.5, 64.0, -2.5));
        assert_eq!(velocities.len(), 8);
        assert!(velocities.iter().all(|v| (0.0..=0.2).contains(&v.y)));
        let ring = shatter_ring([10, 64, -3]);
        assert_eq!(ring.len(), 80);
        for (start, motion) in &ring {
            let out = *start - DVec3::new(10.5, 63.6, -2.5);
            assert!((out.length() - 5.0).abs() < 1.0e-9);
            assert_eq!(motion.y, 0.0);
            let speed = motion.length();
            assert!((speed - 5.0).abs() < 1.0e-9 || (speed - 7.0).abs() < 1.0e-9);
        }
    }

    #[test]
    fn the_trail_starts_half_a_block_below_and_a_quarter_step_back() {
        let view = EyeView {
            owner: 0,
            previous: DVec3::ZERO,
            position: DVec3::new(4.0, 70.0, 4.0),
            motion: DVec3::new(0.4, 0.2, 0.0),
            age: 3,
        };
        let mut random = LegacyRandom::new(9);
        for _ in 0..50 {
            let (start, motion) = trail(&view, &mut random);
            assert_eq!(motion, view.motion);
            assert!((start.y - 69.45).abs() < 1.0e-9);
            assert!((start.x - 3.9).abs() <= 0.3 && (start.z - 4.0).abs() <= 0.3);
        }
    }

    fn registries() -> Option<Arc<Registries>> {
        let paths = DataPaths::discover().ok()?;
        Some(Arc::new(Registries::load(&paths).ok()?))
    }

    fn worldgen(registries: &Arc<Registries>, dimension: &str) -> Option<WorldGen> {
        let terrain = if dimension == "minecraft:the_nether" {
            TerrainGenerator::nether(registries.clone(), 0).ok()?
        } else {
            TerrainGenerator::overworld(registries.clone(), 0).ok()?
        };
        WorldGen::for_dimension(Arc::new(terrain), dimension).ok()
    }

    #[test]
    fn a_thrown_eye_sounds_flies_and_then_drops_or_shatters() {
        let Some(registries) = registries() else {
            return;
        };
        let Some(gen) = worldgen(&registries, "minecraft:overworld") else {
            return;
        };
        let (mut dropped, mut shattered) = (0, 0);
        for seed in 0..12 {
            let mut level = Level::new(&gen.library, -64, 384);
            let mut eyes = Eyes::seeded(seed);
            let mut throw = throw_at(DVec3::new(0.5, 100.9, 0.5));
            throw.components = Some(r#"{"minecraft:custom_name":"Seeker"}"#.to_owned());
            eyes.launch(&mut level, 4, &throw, DVec3::new(1600.0, 0.0, -800.0));
            let launch = std::mem::take(&mut level.sounds);
            assert_eq!(launch.len(), 1);
            assert_eq!(launch[0].event, LAUNCH_SOUND);
            assert_eq!(launch[0].pos, throw.feet.to_array());
            assert!((0.33..=0.5).contains(&launch[0].pitch));
            for _ in 0..LIFE {
                eyes.tick(&mut level);
            }
            let flying = eyes.snapshot();
            assert_eq!(flying.views.len(), 1);
            let view = flying.views[0];
            assert_eq!(view.owner, 4);
            assert!(view.position.x > 0.5 && view.position.z < 0.5);
            assert!(view.position.y > throw.middle.y);
            assert!(level.sounds.is_empty());
            let survives = eyes.flying()[0].survives;
            eyes.tick(&mut level);
            let after = eyes.snapshot();
            assert!(after.views.is_empty());
            let end = view.position + view.motion;
            assert_eq!(level.sounds.len(), 1);
            assert_eq!(level.sounds[0].event, DEATH_SOUND);
            assert!(close(DVec3::from_array(level.sounds[0].pos), end));
            let items: Vec<_> = level
                .entities
                .iter()
                .filter_map(|e| match &e.kind {
                    EntityKind::Item(data) => Some((e.pos, e.delta, data.stack.clone())),
                    _ => None,
                })
                .collect();
            if survives {
                dropped += 1;
                assert!(after.shattered.is_empty());
                assert_eq!(items.len(), 1);
                let (pos, delta, stack) = &items[0];
                assert!(close(DVec3::from_array(*pos), end));
                assert_eq!(delta[1], 0.2);
                assert!(delta[0].abs() <= 0.1 && delta[2].abs() <= 0.1);
                assert_eq!((stack.id.as_str(), stack.count), (ENDER_EYE, 1));
                assert!(stack.components.is_some(), "the eye keeps its name");
            } else {
                shattered += 1;
                assert!(items.is_empty());
                assert_eq!(after.shattered, vec![block_of(end)]);
            }
            assert!(eyes.snapshot().shattered.is_empty(), "a break is sent once");
        }
        assert!(
            dropped > 0 && shattered > 0,
            "{dropped} dropped, {shattered} shattered"
        );
    }

    #[test]
    #[ignore = "resolves all 128 stronghold rings"]
    fn an_overworld_eye_finds_a_stronghold_ring_chunk_corner() {
        let Some(registries) = registries() else {
            return;
        };
        let Some(overworld) = worldgen(&registries, "minecraft:overworld") else {
            return;
        };
        let started = Instant::now();
        let found = Eyes::seeded(0).locate(&overworld, DVec3::new(0.5, 64.0, 0.5));
        let found = found.expect("seed 0 has strongholds");
        assert_eq!(found.y, 0.0);
        assert_eq!((found.x as i32 % 16, found.z as i32 % 16), (0, 0));
        assert!(started.elapsed() < SEARCH_CAP, "{:?}", started.elapsed());
    }

    #[test]
    fn eyes_of_ender_look_for_the_stronghold_and_find_none_in_the_nether() {
        let Some(registries) = registries() else {
            return;
        };
        let Some(overworld) = worldgen(&registries, "minecraft:overworld") else {
            return;
        };
        assert_eq!(
            located_structures(&overworld),
            vec!["minecraft:stronghold".to_owned()]
        );
        let Some(nether) = worldgen(&registries, "minecraft:the_nether") else {
            return;
        };
        let mut eyes = Eyes::seeded(0);
        assert_eq!(eyes.locate(&nether, DVec3::new(0.5, 64.0, 0.5)), None);
        let Ok(states) = crate::terrain::BlockStates::new(registries.clone(), 0, 0, 256) else {
            return;
        };
        let mut server = crate::server::ServerSim::new(
            Arc::new(nether),
            Arc::new(states),
            "minecraft:the_nether",
        );
        assert!(!server.throw_eye(0, &throw_at(DVec3::new(0.5, 64.9, 0.5))));
        assert!(server.eye_snapshot().views.is_empty());
    }
}
