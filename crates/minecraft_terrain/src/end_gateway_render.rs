use glam::{DVec3, Mat4, Vec3};

use crate::end_crystal_render::beam_spans;
use crate::end_gateway::{COOLDOWN_TIME, GatewayView, Pos, SPAWN_TIME};
use crate::mesh::{Atlas, ChunkMesh, Vertex};
use crate::pack::ResourceId;

pub const BEAM_TEXTURE: &str = "minecraft:entity/end_portal/end_gateway_beam";
pub const VIEW_DISTANCE: f64 = 256.0;
pub const MAGENTA: u32 = 13_061_821;
pub const PURPLE: u32 = 8_991_416;
const BEAM_RADIUS: f32 = 0.15;
const GLOW_RADIUS: f32 = 0.175;
pub const INNER_REPEAT: f32 = 0.5 / BEAM_RADIUS;
pub const GLOW_REPEAT: f32 = 1.0;
const GLOW_ALPHA: u8 = 32;
const COOLDOWN_HEIGHT: f64 = 50.0;
const ANIMATION_LOOP: i64 = 40;
const FULL_BRIGHT: f32 = 15.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Beam {
    pub scale: f32,
    pub height: i32,
    pub color: u32,
    pub animation_time: f32,
}

pub fn spawn_percent(age: i64, partial: f32) -> f32 {
    ((age as f32 + partial) / SPAWN_TIME as f32).clamp(0.0, 1.0)
}

pub fn cooldown_percent(cooldown: i32, partial: f32) -> f32 {
    1.0 - ((cooldown as f32 - partial) / COOLDOWN_TIME as f32).clamp(0.0, 1.0)
}

pub fn beam(view: &GatewayView, max_y: i32, game_time: i64, partial: f32) -> Option<Beam> {
    let spawning = view.is_spawning();
    if !spawning && !view.is_cooling_down() {
        return None;
    }
    let percent = if spawning {
        spawn_percent(view.age, partial)
    } else {
        cooldown_percent(view.cooldown, partial)
    };
    let scale = minecraftoss_player::mth::sin(f64::from(percent * std::f32::consts::PI));
    let top = if spawning {
        f64::from(max_y)
    } else {
        COOLDOWN_HEIGHT
    };
    Some(Beam {
        scale,
        height: (f64::from(scale) * top).floor() as i32,
        color: if spawning { MAGENTA } else { PURPLE },
        animation_time: game_time.rem_euclid(ANIMATION_LOOP) as f32 + partial,
    })
}

fn frac(value: f32) -> f32 {
    value - value.floor()
}

pub fn beam_v(beam: &Beam, repeat: f32) -> (f32, f32) {
    let t = -beam.animation_time;
    let start = -1.0 + frac(t * 0.2 - (t * 0.1).floor());
    let length = (beam.height * 2) as f32 * beam.scale;
    (start, length * repeat + start)
}

pub fn beam_turn(beam: &Beam) -> f32 {
    (beam.animation_time * 2.25 - 45.0).to_radians()
}

fn rgb(color: u32) -> [f32; 3] {
    [
        ((color >> 16) & 0xff) as f32 / 255.0,
        ((color >> 8) & 0xff) as f32 / 255.0,
        (color & 0xff) as f32 / 255.0,
    ]
}

struct Part {
    pose: Mat4,
    corners: [(f32, f32); 4],
    y: (f32, f32),
    v: (f32, f32),
    color: [f32; 4],
}

fn quad(
    mesh: &mut ChunkMesh,
    part: &Part,
    first: (f32, f32),
    second: (f32, f32),
    region: [f32; 4],
) {
    let (bottom, top) = part.y;
    let (v_bottom, v_top) = part.v;
    for (s0, s1) in beam_spans((v_bottom, v_top)) {
        let v_at = |s: f32| v_bottom + (v_top - v_bottom) * s;
        let floor = v_at((s0 + s1) / 2.0).floor();
        let vertex = |s: f32, (x, z): (f32, f32), u: f32| {
            let point = part
                .pose
                .transform_point3(Vec3::new(x, bottom + (top - bottom) * s, z));
            let v = (v_at(s) - floor).clamp(0.0, 1.0);
            Vertex {
                position: point.to_array(),
                uv: [
                    region[0] + (region[2] - region[0]) * u,
                    region[1] + (region[3] - region[1]) * v,
                ],
                color: part.color,
                sky_light: FULL_BRIGHT,
                block_light: FULL_BRIGHT,
            }
        };
        let start = mesh.vertices.len() as u32;
        mesh.vertices.extend([
            vertex(s1, first, 1.0),
            vertex(s0, first, 1.0),
            vertex(s0, second, 0.0),
            vertex(s1, second, 0.0),
        ]);
        mesh.indices
            .extend_from_slice(&[start, start + 1, start + 2, start, start + 2, start + 3]);
        mesh.faces += 1;
    }
}

fn part(mesh: &mut ChunkMesh, part: &Part, region: [f32; 4]) {
    let [p1, p2, p3, p4] = part.corners;
    for (first, second) in [(p1, p2), (p4, p3), (p2, p4), (p3, p1)] {
        quad(mesh, part, first, second, region);
    }
}

pub fn append_beam(
    models: &mut ChunkMesh,
    translucent: &mut ChunkMesh,
    (x, y, z): Pos,
    beam: &Beam,
    region: [f32; 4],
) {
    if beam.height <= 0 {
        return;
    }
    let base = Mat4::from_translation(Vec3::new(x as f32 + 0.5, y as f32, z as f32 + 0.5));
    let height = beam.height as f32;
    let [r, g, b] = rgb(beam.color);
    let inner = BEAM_RADIUS;
    part(
        models,
        &Part {
            pose: base * Mat4::from_rotation_y(beam_turn(beam)),
            corners: [(0.0, inner), (inner, 0.0), (-inner, 0.0), (0.0, -inner)],
            y: (-height, height),
            v: beam_v(beam, INNER_REPEAT),
            color: [r, g, b, 1.0],
        },
        region,
    );
    let glow = GLOW_RADIUS;
    part(
        translucent,
        &Part {
            pose: base,
            corners: [(-glow, -glow), (glow, -glow), (-glow, glow), (glow, glow)],
            y: (-height, height),
            v: beam_v(beam, GLOW_REPEAT),
            color: [r, g, b, f32::from(GLOW_ALPHA) / 255.0],
        },
        region,
    );
}

#[allow(clippy::too_many_arguments)]
pub fn append_beams<'a>(
    models: &mut ChunkMesh,
    translucent: &mut ChunkMesh,
    views: impl IntoIterator<Item = &'a GatewayView>,
    atlas: &Atlas,
    camera: DVec3,
    max_y: i32,
    game_time: i64,
    partial: f32,
) {
    let Some(region) = ResourceId::parse(BEAM_TEXTURE)
        .ok()
        .filter(|id| atlas.contains(id))
        .map(|id| atlas.entity_region(&id))
    else {
        return;
    };
    for view in views {
        let (x, y, z) = view.pos;
        let center = DVec3::new(f64::from(x) + 0.5, f64::from(y) + 0.5, f64::from(z) + 0.5);
        if center.distance(camera) > VIEW_DISTANCE {
            continue;
        }
        if let Some(beam) = beam(view, max_y, game_time, partial) {
            append_beam(models, translucent, view.pos, &beam, region);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gateway(age: i64, cooldown: i32) -> GatewayView {
        GatewayView {
            pos: (10, 75, -4),
            age,
            cooldown,
            exit: None,
            exact: false,
        }
    }

    #[test]
    fn the_spawn_beam_rises_to_the_build_height_and_falls_over_ten_seconds() {
        let at = |age: i64| beam(&gateway(age, 0), 255, 0, 0.0).expect("a spawning beam");
        assert_eq!(at(0).height, 0);
        assert_eq!(at(100).height, 255);
        assert_eq!(at(199).height, 4);
        assert_eq!(at(50).height, (255.0 * at(50).scale) as i32);
        assert!((at(50).scale - std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
        assert_eq!(at(100).color, MAGENTA);
        assert_eq!(beam(&gateway(200, 0), 255, 0, 0.0), None);
        let half = beam(&gateway(99, 0), 255, 0, 1.0).expect("a spawning beam");
        assert_eq!(half.height, 255, "the partial tick counts");
    }

    #[test]
    fn the_cooldown_beam_is_purple_and_fifty_high_at_its_peak() {
        let at =
            |cooldown: i32| beam(&gateway(5000, cooldown), 255, 0, 0.0).expect("a cooling beam");
        assert_eq!(at(20).height, 50);
        assert_eq!(at(20).color, PURPLE);
        assert_eq!(at(40).height, 0);
        assert_eq!(at(1).height, (50.0 * at(1).scale).floor() as i32);
        assert!(at(1).height < 8);
        let spawning_and_cooling = beam(&gateway(100, 20), 255, 0, 0.0).expect("a beam");
        assert_eq!(
            (spawning_and_cooling.color, spawning_and_cooling.height),
            (MAGENTA, 255),
            "spawning wins"
        );
    }

    #[test]
    fn the_beam_scrolls_and_turns_with_game_time() {
        let at = |game_time: i64, partial: f32| {
            beam(&gateway(100, 0), 255, game_time, partial).expect("a beam")
        };
        assert_eq!(at(0, 0.0).animation_time, 0.0);
        assert_eq!(at(41, 0.5).animation_time, 1.5);
        assert_eq!(at(-1, 0.0).animation_time, 39.0);
        let still = at(0, 0.0);
        assert_eq!(beam_v(&still, INNER_REPEAT).0, -1.0);
        assert!((beam_turn(&still) + std::f32::consts::FRAC_PI_4).abs() < 1e-6);
        let later = at(2, 0.5);
        let (start, end) = beam_v(&later, INNER_REPEAT);
        assert!((start - (-1.0 + 0.5)).abs() < 1e-5, "{start}");
        let length = (later.height * 2) as f32 * later.scale;
        assert_eq!(later.height, 254, "half a tick past the peak");
        assert!((end - start - length / 0.3).abs() < 1e-2, "{end}");
        let (glow_start, glow_end) = beam_v(&later, GLOW_REPEAT);
        assert_eq!(glow_start, start);
        assert!((glow_end - glow_start - length).abs() < 1e-3);
        assert!((beam_turn(&later) - (2.5f32 * 2.25 - 45.0).to_radians()).abs() < 1e-6);
    }

    #[test]
    fn the_beam_is_an_opaque_turned_core_inside_a_faint_glow() {
        let (mut models, mut translucent) = (ChunkMesh::default(), ChunkMesh::default());
        let short = Beam {
            scale: 1.0,
            height: 1,
            color: PURPLE,
            animation_time: 0.0,
        };
        append_beam(
            &mut models,
            &mut translucent,
            (10, 75, -4),
            &short,
            [0.0, 0.0, 1.0, 1.0],
        );
        assert!(models.faces >= 4 && translucent.faces >= 4);
        let ys: Vec<f32> = models.vertices.iter().map(|v| v.position[1]).collect();
        assert!(
            ys.iter().all(|&y| (74.0 - 1e-4..=76.0 + 1e-4).contains(&y)),
            "{ys:?}"
        );
        assert!(
            models
                .vertices
                .iter()
                .all(|v| v.color[3] == 1.0 && v.sky_light == 15.0)
        );
        assert!(
            translucent
                .vertices
                .iter()
                .all(|v| (v.color[3] - 32.0 / 255.0).abs() < 1e-6)
        );
        let reach = |mesh: &ChunkMesh| {
            mesh.vertices
                .iter()
                .map(|v| {
                    (v.position[0] - 10.5)
                        .abs()
                        .max((v.position[2] + 3.5).abs())
                })
                .fold(0.0f32, f32::max)
        };
        assert!((reach(&translucent) - GLOW_RADIUS).abs() < 1e-5);
        assert!(reach(&models) <= BEAM_RADIUS + 1e-5);
        assert!(
            models
                .vertices
                .iter()
                .all(|v| (0.0..=1.0).contains(&v.uv[1]))
        );
        let none = Beam { height: 0, ..short };
        let (mut empty, mut glow) = (ChunkMesh::default(), ChunkMesh::default());
        append_beam(
            &mut empty,
            &mut glow,
            (0, 0, 0),
            &none,
            [0.0, 0.0, 1.0, 1.0],
        );
        assert_eq!((empty.faces, glow.faces), (0, 0));
    }
}
