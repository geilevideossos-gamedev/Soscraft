//! Grind rails found in the map's collision: the lips a skater can grind.
//!
//! MW2 collision comes from brushes, the clip mesh and static model collision,
//! so the top of a ledge and its wall rarely share an edge vertex for vertex.
//! A lip is therefore found by probing the geometry around each edge of a
//! walkable face rather than by matching triangles: the ground must fall away
//! just past the edge and nothing may rise there. Surviving edges are merged
//! along their lines across seams and T-junctions, then chained into
//! polylines where they meet at a gentle turn, as the authoring tool does for
//! edges marked sharp.

use std::collections::HashMap;

use bevy::math::Vec3;

/// A face this upward is walkable; its edges are rail candidates.
const UPWARD_Z: f32 = 0.65;
/// XY grid cell for probing, in map units (inches).
const CELL: f32 = 64.0;
/// How far past the lip the probes look.
const PROBE_OUT: f32 = 2.0;
/// Open space the ground must drop below the lip, just past it.
const MIN_DROP: f32 = 3.0;
/// Height above the lip the wall probe runs at.
const WALL_PROBE_UP: f32 = 3.0;
/// Shortest rail kept, after merging and chaining.
const MIN_RAIL: f32 = 24.0;
/// Steepest rail, as rise over length: stair handrails and ramp edges pass.
const MAX_SLOPE: f32 = 0.7;
/// Chains continue through a joint turning less than about 35 degrees.
const MIN_TURN_COS: f32 = 0.82;
/// The skate engine counts rails in a u16.
const MAX_RAILS: usize = u16::MAX as usize;
const RIDGE_WIDTH: f32 = 18.25;
const RIDGE_HEIGHT: f32 = 0.5;
const RIDGE_MIN_OVERLAP: f32 = 1.0;
const MIN_STEEP_RISE: f32 = 18.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limits {
    pub max_slope: f32,
    pub merge_ridges: bool,
}

impl Limits {
    pub const MW2: Limits = Limits {
        max_slope: MAX_SLOPE,
        merge_ridges: false,
    };
    pub const BLOCKS: Limits = Limits {
        max_slope: 1.01,
        merge_ridges: true,
    };
}

pub struct RailCensus {
    pub candidates: usize,
    pub lips: usize,
    pub runs: usize,
    pub rails: usize,
}

/// Rails over `tris` (map units, z up), each a polyline of two or more points.
pub fn find(tris: &[[Vec3; 3]], limits: Limits) -> (Vec<Vec<Vec3>>, RailCensus) {
    let grid = Grid::build(tris);
    let mut probe = Probe {
        tris,
        grid: &grid,
        stamp: vec![0; tris.len()],
        round: 0,
        scratch: Vec::new(),
    };

    // Every edge of a walkable face, once.
    let key = |v: Vec3| v.to_array().map(|x| (x * 8.).round() as i32);
    let mut edges: HashMap<([i32; 3], [i32; 3]), (Vec3, Vec3, Vec3, Vec3)> = HashMap::new();
    for tri in tris {
        let normal = (tri[1] - tri[0]).cross(tri[2] - tri[0]).normalize_or_zero();
        if normal.z <= UPWARD_Z {
            continue;
        }
        let centroid = (tri[0] + tri[1] + tri[2]) / 3.;
        for i in 0..3 {
            let (a, b) = (tri[i], tri[(i + 1) % 3]);
            let (ka, kb) = (key(a), key(b));
            let k = if ka < kb { (ka, kb) } else { (kb, ka) };
            edges.entry(k).or_insert((a, b, normal, centroid));
        }
    }
    let candidates = edges.len();

    let mut lips = Vec::new();
    for (a, b, normal, centroid) in edges.into_values() {
        let along = b - a;
        let len = along.length();
        if len < 1. || along.z.abs() > limits.max_slope * len {
            continue;
        }
        let mid = (a + b) * 0.5;
        let mut out = along.cross(normal).with_z(0.).normalize_or_zero();
        if out.dot(centroid - mid) > 0. {
            out = -out;
        }
        let rise = -(normal.x * out.x + normal.y * out.y) / normal.z;
        let samples: &[f32] = if len < 12. {
            &[0.5]
        } else {
            &[0.25, 0.5, 0.75]
        };
        let passing = samples
            .iter()
            .filter(|&&s| probe.is_lip(a + along * s, out, rise))
            .count();
        if passing * 3 >= samples.len() * 2 {
            lips.push((a, b, out));
        }
    }
    let lip_count = lips.len();

    let runs = if limits.merge_ridges {
        merge_ridges(lips)
    } else {
        merge_collinear(lips.into_iter().map(|(a, b, _)| (a, b)).collect())
    };
    let run_count = runs.len();
    let mut rails = chain(runs);
    rails.retain(|rail| polyline_len(rail) >= MIN_RAIL && !short_and_steep(rail));
    if rails.len() > MAX_RAILS {
        rails.sort_by(|a, b| polyline_len(b).total_cmp(&polyline_len(a)));
        rails.truncate(MAX_RAILS);
    }
    let census = RailCensus {
        candidates,
        lips: lip_count,
        runs: run_count,
        rails: rails.len(),
    };
    (rails, census)
}

fn polyline_len(points: &[Vec3]) -> f32 {
    points.windows(2).map(|w| w[0].distance(w[1])).sum()
}

fn short_and_steep(points: &[Vec3]) -> bool {
    let steep = points
        .windows(2)
        .any(|w| (w[1].z - w[0].z).abs() > MAX_SLOPE * w[0].distance(w[1]));
    let (low, high) = points
        .iter()
        .fold((f32::MAX, f32::MIN), |(low, high), p| (low.min(p.z), high.max(p.z)));
    steep && high - low < MIN_STEEP_RISE
}

struct Grid {
    cells: HashMap<(i32, i32), Vec<u32>>,
}

fn cell_of(v: f32) -> i32 {
    (v / CELL).floor() as i32
}

impl Grid {
    fn build(tris: &[[Vec3; 3]]) -> Self {
        let mut cells: HashMap<(i32, i32), Vec<u32>> = HashMap::new();
        for (index, tri) in tris.iter().enumerate() {
            let min = tri[0].min(tri[1]).min(tri[2]);
            let max = tri[0].max(tri[1]).max(tri[2]);
            for x in cell_of(min.x)..=cell_of(max.x) {
                for y in cell_of(min.y)..=cell_of(max.y) {
                    cells.entry((x, y)).or_default().push(index as u32);
                }
            }
        }
        Self { cells }
    }
}

struct Probe<'a> {
    tris: &'a [[Vec3; 3]],
    grid: &'a Grid,
    stamp: Vec<u32>,
    round: u32,
    scratch: Vec<u32>,
}

impl Probe<'_> {
    /// Past `p` along `out` the ground falls away and nothing rises.
    fn is_lip(&mut self, p: Vec3, out: Vec3, rise: f32) -> bool {
        let lift = 1. + (rise * PROBE_OUT).max(0.);
        let down_from = p + out * PROBE_OUT + Vec3::Z * lift;
        if self.hit(down_from, -Vec3::Z, lift + MIN_DROP) {
            return false;
        }
        let across_from = p - out + Vec3::Z * WALL_PROBE_UP;
        !self.hit(across_from, out, 2. + PROBE_OUT)
    }

    /// Whether the segment from `origin` along unit `dir` for `len` crosses
    /// any triangle, either side facing.
    fn hit(&mut self, origin: Vec3, dir: Vec3, len: f32) -> bool {
        let end = origin + dir * len;
        let (min, max) = (origin.min(end), origin.max(end));
        self.round = self.round.wrapping_add(1);
        if self.round == 0 {
            self.stamp.fill(0);
            self.round = 1;
        }
        self.scratch.clear();
        for x in cell_of(min.x)..=cell_of(max.x) {
            for y in cell_of(min.y)..=cell_of(max.y) {
                let Some(cell) = self.grid.cells.get(&(x, y)) else {
                    continue;
                };
                for &index in cell {
                    let seen = &mut self.stamp[index as usize];
                    if *seen != self.round {
                        *seen = self.round;
                        self.scratch.push(index);
                    }
                }
            }
        }
        self.scratch
            .iter()
            .any(|&index| ray_triangle(origin, dir, len, &self.tris[index as usize]))
    }
}

/// Möller–Trumbore, both faces, hits within `(0, len]`.
fn ray_triangle(origin: Vec3, dir: Vec3, len: f32, tri: &[Vec3; 3]) -> bool {
    let e1 = tri[1] - tri[0];
    let e2 = tri[2] - tri[0];
    let p = dir.cross(e2);
    let det = e1.dot(p);
    if det.abs() < 1e-8 {
        return false;
    }
    let inv = 1. / det;
    let s = origin - tri[0];
    let u = s.dot(p) * inv;
    if !(0. ..=1.).contains(&u) {
        return false;
    }
    let q = s.cross(e1);
    let v = dir.dot(q) * inv;
    if v < 0. || u + v > 1. {
        return false;
    }
    let t = e2.dot(q) * inv;
    t > 1e-4 && t <= len
}

/// Lips on one line, merged into maximal runs where they overlap or touch.
fn merge_collinear(lips: Vec<(Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    let mut lines: HashMap<([i32; 3], [i32; 3]), (Vec3, Vec3, Vec<(f32, f32)>)> = HashMap::new();
    for (mut a, mut b) in lips {
        let mut d = (b - a).normalize_or_zero();
        if d == Vec3::ZERO {
            continue;
        }
        let flip =
            d.x < -1e-4 || (d.x.abs() <= 1e-4 && (d.y < -1e-4 || (d.y.abs() <= 1e-4 && d.z < 0.)));
        if flip {
            d = -d;
            std::mem::swap(&mut a, &mut b);
        }
        let o = a - d * a.dot(d);
        let k = (
            (d * 64.).to_array().map(|x| x.round() as i32),
            (o * 2.).to_array().map(|x| x.round() as i32),
        );
        let line = lines.entry(k).or_insert((d, o, Vec::new()));
        line.2.push((a.dot(line.0), b.dot(line.0)));
    }
    let mut runs = Vec::new();
    for (d, o, mut spans) in lines.into_values() {
        spans.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut current = spans[0];
        for &(t0, t1) in &spans[1..] {
            if t0 <= current.1 + 1.5 {
                current.1 = current.1.max(t1);
            } else {
                runs.push((o + d * current.0, o + d * current.1));
                current = (t0, t1);
            }
        }
        runs.push((o + d * current.0, o + d * current.1));
    }
    runs
}

fn merge_ridges(lips: Vec<(Vec3, Vec3, Vec3)>) -> Vec<(Vec3, Vec3)> {
    let mut sides: HashMap<[i32; 2], (Vec3, Vec<(Vec3, Vec3)>)> = HashMap::new();
    for (a, b, out) in lips {
        let key = [out.x, out.y].map(|v| (v * 64.).round() as i32);
        sides.entry(key).or_insert((out, Vec::new())).1.push((a, b));
    }
    let mut runs = Vec::new();
    for (out, side) in sides.into_values() {
        runs.extend(
            merge_collinear(side)
                .into_iter()
                .map(|(a, b)| (a, b, out)),
        );
    }

    let mut lines: HashMap<[i32; 3], Vec<(f32, usize)>> = HashMap::new();
    for (index, &(a, b, _)) in runs.iter().enumerate() {
        let d = (b - a).normalize_or_zero();
        let across = Vec3::new(-d.y, d.x, 0.).normalize_or_zero();
        let key = d.to_array().map(|v| (v * 64.).round() as i32);
        lines.entry(key).or_default().push((a.dot(across), index));
    }
    let mut ridges = Vec::new();
    for mut line in lines.into_values() {
        line.sort_by(|x, y| x.0.total_cmp(&y.0));
        for (k, &(offset, i)) in line.iter().enumerate() {
            for &(other, j) in &line[k + 1..] {
                if other - offset > RIDGE_WIDTH {
                    break;
                }
                if let Some(ridge) = ridge(runs[i], runs[j]) {
                    ridges.push((i, j, ridge));
                }
            }
        }
    }
    ridges.sort_by(|x, y| x.2.width.total_cmp(&y.2.width));

    let mut covered: Vec<Vec<(f32, f32)>> = vec![Vec::new(); runs.len()];
    let clashes = |spans: &[(f32, f32)], lo: f32, hi: f32| {
        spans
            .iter()
            .any(|&(s0, s1)| s1.min(hi) - s0.max(lo) > RIDGE_MIN_OVERLAP)
    };
    let mut merged = Vec::new();
    for (i, j, ridge) in ridges {
        let ((i0, i1), (j0, j1)) = (ridge.first, ridge.second);
        if clashes(&covered[i], i0, i1) || clashes(&covered[j], j0, j1) {
            continue;
        }
        covered[i].push(ridge.first);
        covered[j].push(ridge.second);
        merged.push((ridge.start, ridge.end));
    }
    for (&(a, b, _), mut spans) in runs.iter().zip(covered) {
        let d = (b - a).normalize_or_zero();
        let base = a.dot(d);
        let at = |t: f32| a + d * (t - base);
        spans.sort_by(|x, y| x.0.total_cmp(&y.0));
        let mut from = base;
        for (s0, s1) in spans {
            if s0 - from >= RIDGE_MIN_OVERLAP {
                merged.push((at(from), at(s0)));
            }
            from = from.max(s1);
        }
        if b.dot(d) - from >= RIDGE_MIN_OVERLAP {
            merged.push((at(from), b));
        }
    }
    merge_collinear(merged)
}

struct Ridge {
    width: f32,
    first: (f32, f32),
    second: (f32, f32),
    start: Vec3,
    end: Vec3,
}

fn ridge(first: (Vec3, Vec3, Vec3), second: (Vec3, Vec3, Vec3)) -> Option<Ridge> {
    let (a, b, out) = first;
    let (c, e, other_out) = second;
    if out.dot(other_out) > -0.99 {
        return None;
    }
    let d = (b - a).normalize_or_zero();
    let other_d = (e - c).normalize_or_zero();
    if d.dot(other_d) < 0.999 {
        return None;
    }
    let gap = c - a;
    let w = gap - d * gap.dot(d);
    if w.z.abs() > RIDGE_HEIGHT
        || w.length() > RIDGE_WIDTH
        || w.dot(out) >= 0.
        || w.dot(other_out) <= 0.
    {
        return None;
    }
    let lo = a.dot(d).max(c.dot(d));
    let hi = b.dot(d).min(e.dot(d));
    if hi - lo < RIDGE_MIN_OVERLAP {
        return None;
    }
    let base = a.dot(d);
    let (near, far) = (a + d * (lo - base), a + d * (hi - base));
    Some(Ridge {
        width: w.length(),
        first: (lo, hi),
        second: ((near + w).dot(other_d), (far + w).dot(other_d)),
        start: near + w * 0.5,
        end: far + w * 0.5,
    })
}

/// Runs joined end to end into polylines through joints where exactly two
/// runs meet at a gentle turn.
fn chain(runs: Vec<(Vec3, Vec3)>) -> Vec<Vec<Vec3>> {
    let node = |v: Vec3| v.to_array().map(|x| x.round() as i32);
    let mut at: HashMap<[i32; 3], Vec<(usize, bool)>> = HashMap::new();
    for (i, (a, b)) in runs.iter().enumerate() {
        at.entry(node(*a)).or_default().push((i, false));
        at.entry(node(*b)).or_default().push((i, true));
    }
    let mut used = vec![false; runs.len()];
    let far = |i: usize, from_end: bool| if from_end { runs[i].0 } else { runs[i].1 };
    let near = |i: usize, from_end: bool| if from_end { runs[i].1 } else { runs[i].0 };
    // The run continuing a polyline at `joint`, arriving along `heading`.
    let next = |joint: Vec3, heading: Vec3, used: &[bool]| -> Option<(usize, bool)> {
        let there = at.get(&node(joint))?;
        if there.len() != 2 {
            return None;
        }
        let &(i, at_end) = there.iter().find(|(i, _)| !used[*i])?;
        let leave = (far(i, at_end) - near(i, at_end)).normalize_or_zero();
        (heading.dot(leave) >= MIN_TURN_COS).then_some((i, at_end))
    };
    let mut rails = Vec::new();
    for start in 0..runs.len() {
        if used[start] {
            continue;
        }
        used[start] = true;
        let (a, b) = runs[start];
        let mut points = std::collections::VecDeque::from([a, b]);
        // Forward from b, then backward from a.
        let mut tip = b;
        let mut heading = (b - a).normalize_or_zero();
        while let Some((i, at_end)) = next(tip, heading, &used) {
            used[i] = true;
            let to = far(i, at_end);
            heading = (to - tip).normalize_or_zero();
            tip = to;
            points.push_back(to);
        }
        let mut tip = a;
        let mut heading = (a - b).normalize_or_zero();
        while let Some((i, at_end)) = next(tip, heading, &used) {
            used[i] = true;
            let to = far(i, at_end);
            heading = (to - tip).normalize_or_zero();
            tip = to;
            points.push_front(to);
        }
        rails.push(points.into_iter().collect());
    }
    rails
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cuboid(min: Vec3, max: Vec3) -> Vec<[Vec3; 3]> {
        let (lo, hi) = (min.to_array(), max.to_array());
        let mut tris = Vec::new();
        for axis in 0..3 {
            let (u, v) = [(1, 2), (2, 0), (0, 1)][axis];
            for high in [false, true] {
                let corner = |a: usize, c: usize| {
                    let mut p = [0.0; 3];
                    p[axis] = if high { hi[axis] } else { lo[axis] };
                    p[u] = [lo[u], hi[u]][a];
                    p[v] = [lo[v], hi[v]][c];
                    Vec3::from_array(p)
                };
                let mut quad = [corner(0, 0), corner(1, 0), corner(1, 1), corner(0, 1)];
                if !high {
                    quad.reverse();
                }
                tris.push([quad[0], quad[1], quad[2]]);
                tris.push([quad[0], quad[2], quad[3]]);
            }
        }
        tris
    }

    fn ground() -> Vec<[Vec3; 3]> {
        let corners = [
            Vec3::new(-400., -400., 0.),
            Vec3::new(800., -400., 0.),
            Vec3::new(800., 400., 0.),
            Vec3::new(-400., 400., 0.),
        ];
        vec![
            [corners[0], corners[1], corners[2]],
            [corners[0], corners[2], corners[3]],
        ]
    }

    fn wall(width: f32, height: f32) -> Vec<[Vec3; 3]> {
        let mut tris = ground();
        tris.extend(cuboid(
            Vec3::new(0., -width / 2., 0.),
            Vec3::new(360., width / 2., height),
        ));
        tris
    }

    fn long_rails(tris: &[[Vec3; 3]], limits: Limits) -> Vec<Vec<Vec3>> {
        let (found, _) = find(tris, limits);
        found
            .into_iter()
            .filter(|rail| polyline_len(rail) > 300.)
            .filter(|rail| rail.iter().all(|p| p.y.abs() < 100. && p.x > -50. && p.x < 410.))
            .collect()
    }

    #[test]
    fn two_close_parallel_lips_become_one_centre_rail() {
        for (width, height) in [(4.5, 36.), (9., 54.), (13.5, 54.), (18., 54.)] {
            let tris = wall(width, height);
            assert_eq!(long_rails(&tris, Limits::MW2).len(), 2, "{width}");
            let rails = long_rails(&tris, Limits::BLOCKS);
            assert_eq!(rails.len(), 1, "{width}");
            for p in &rails[0] {
                assert!(p.y.abs() < 0.01 && (p.z - height).abs() < 0.01, "{width}: {p}");
            }
            assert!((polyline_len(&rails[0]) - 360.).abs() < 0.5, "{width}");
        }
    }

    #[test]
    fn a_wall_post_and_its_sides_make_one_centre_rail() {
        let mut tris = wall(13.5, 54.);
        tris.extend(cuboid(
            Vec3::new(171., -9., 0.),
            Vec3::new(189., 9., 54.),
        ));
        let rails = long_rails(&tris, Limits::BLOCKS);
        assert_eq!(rails.len(), 1);
        assert!(rails[0].iter().all(|p| p.y.abs() < 0.01));
    }

    #[test]
    fn a_one_block_wall_keeps_two_rails() {
        let rails = long_rails(&wall(36., 36.), Limits::BLOCKS);
        assert_eq!(rails.len(), 2);
        let mut sides: Vec<f32> = rails.iter().map(|rail| rail[0].y).collect();
        sides.sort_by(f32::total_cmp);
        assert!((sides[0] + 18.).abs() < 0.01 && (sides[1] - 18.).abs() < 0.01);
    }

    #[test]
    fn lips_facing_each_other_across_a_gap_stay_apart() {
        let mut tris = ground();
        tris.extend(cuboid(Vec3::new(0., -40.5, 0.), Vec3::new(360., -4.5, 36.)));
        tris.extend(cuboid(Vec3::new(0., 4.5, 0.), Vec3::new(360., 40.5, 36.)));
        let rails = long_rails(&tris, Limits::BLOCKS);
        assert_eq!(rails.len(), 4);
        assert!(rails.iter().all(|rail| rail[0].y.abs() > 4.));
    }

    fn ramp(rise: f32) -> Vec<[Vec3; 3]> {
        let mut tris = ground();
        let ramp = [
            Vec3::new(0., 0., 0.),
            Vec3::new(72., 0., 0.),
            Vec3::new(72., rise, rise),
            Vec3::new(0., rise, rise),
        ];
        tris.push([ramp[0], ramp[1], ramp[2]]);
        tris.push([ramp[0], ramp[2], ramp[3]]);
        for x in [0., 72.] {
            tris.push([
                Vec3::new(x, 0., 0.),
                Vec3::new(x, rise, 0.),
                Vec3::new(x, rise, rise),
            ]);
        }
        tris.push([
            Vec3::new(0., rise, 0.),
            Vec3::new(72., rise, 0.),
            Vec3::new(72., rise, rise),
        ]);
        tris.push([
            Vec3::new(0., rise, 0.),
            Vec3::new(72., rise, rise),
            Vec3::new(0., rise, rise),
        ]);
        tris
    }

    fn sloped_rails(tris: &[[Vec3; 3]], limits: Limits) -> usize {
        let (found, _) = find(tris, limits);
        found
            .iter()
            .filter(|rail| (rail[0].z - rail[rail.len() - 1].z).abs() > 9.)
            .count()
    }

    #[test]
    fn the_middle_of_a_sloped_face_is_no_lip() {
        for rise in [18., 36., 72.] {
            let tris = ramp(rise);
            let (found, _) = find(&tris, Limits::MW2);
            assert!(
                found.iter().all(|rail| rail.iter().all(|p| p.z.abs() < 0.01 || (p.z - rise).abs() < 0.01)),
                "{rise}: {found:?}"
            );
        }
    }

    #[test]
    fn mw2_limits_drop_a_45_degree_edge_and_block_limits_keep_it() {
        let tris = ramp(72.);
        assert_eq!(sloped_rails(&tris, Limits::MW2), 0);
        assert_eq!(sloped_rails(&tris, Limits::BLOCKS), 2);
    }

    #[test]
    fn a_45_degree_edge_rising_half_a_block_is_no_rail() {
        assert_eq!(sloped_rails(&ramp(18.), Limits::BLOCKS), 0);
        assert_eq!(sloped_rails(&ramp(36.), Limits::BLOCKS), 2);
    }
}
