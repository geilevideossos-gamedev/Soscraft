use crate::mesh::SectionMesh;
use crate::sections::SectionPos;
use glam::DVec3;
use std::collections::{HashMap, HashSet};

pub const NEARBY_SECTIONS: i32 = 2;

pub fn point_of_view(eye: DVec3, section: SectionPos) -> [i32; 3] {
    let axis = |eye: f64, section: i32| ((eye.floor() as i32 >> 4) - section).clamp(-1, 1);
    [
        axis(eye.x, section.0),
        axis(eye.y, section.1),
        axis(eye.z, section.2),
    ]
}

pub fn axis_aligned(point_of_view: [i32; 3]) -> bool {
    point_of_view.contains(&0)
}

pub fn nearby(eye: DVec3, section: SectionPos) -> bool {
    let camera = |eye: f64| eye.floor() as i32 >> 4;
    (camera(eye.x) - section.0).abs() <= NEARBY_SECTIONS
        && (camera(eye.y) - section.1).abs() <= NEARBY_SECTIONS
        && (camera(eye.z) - section.2).abs() <= NEARBY_SECTIONS
}

#[derive(Clone, Debug, PartialEq)]
pub struct TranslucentQuads {
    start: u32,
    quads: Vec<[u32; 6]>,
    centroids: Vec<[f32; 3]>,
    point_of_view: [i32; 3],
}

impl TranslucentQuads {
    pub fn of(mesh: &SectionMesh) -> Option<Self> {
        let start = mesh.transparent_start?;
        let (quads, rest) = mesh.indices.get(start as usize..)?.as_chunks::<6>();
        if quads.is_empty() || !rest.is_empty() {
            return None;
        }
        let quads = quads.to_vec();
        let position = |index: u32| {
            mesh.vertices
                .get(index as usize)
                .map(|vertex| DVec3::from(vertex.position.map(f64::from)))
        };
        let centroids = quads
            .iter()
            .map(|quad| {
                let first = quad.iter().copied().min().unwrap_or(quad[0]);
                match (position(first), position(first + 2)) {
                    (Some(a), Some(b)) => ((a + b) / 2.0).as_vec3().to_array(),
                    (a, _) => a.unwrap_or(DVec3::ZERO).as_vec3().to_array(),
                }
            })
            .collect();
        Some(Self {
            start,
            quads,
            centroids,
            point_of_view: [i32::MIN; 3],
        })
    }

    pub fn sorted(&self, eye: DVec3) -> Vec<u32> {
        let distances: Vec<f64> = self
            .centroids
            .iter()
            .map(|centroid| DVec3::from(centroid.map(f64::from)).distance_squared(eye))
            .collect();
        let mut order: Vec<usize> = (0..self.quads.len()).collect();
        order.sort_by(|&a, &b| distances[b].total_cmp(&distances[a]));
        order
            .into_iter()
            .flat_map(|quad| self.quads[quad])
            .collect()
    }

    pub fn sort(&mut self, eye: DVec3, section: SectionPos) -> Vec<u32> {
        self.point_of_view = point_of_view(eye, section);
        self.sorted(eye)
    }

    pub fn sort_mesh(&mut self, mesh: &mut SectionMesh, eye: DVec3, section: SectionPos) {
        let sorted = self.sort(eye, section);
        mesh.indices.truncate(self.start as usize);
        mesh.indices.extend(sorted);
    }

    pub fn resort(&mut self, eye: DVec3, section: SectionPos) -> SectionMesh {
        SectionMesh {
            vertices: Vec::new(),
            indices: self.sort(eye, section),
            transparent_start: Some(self.start),
        }
    }

    pub fn wants_resort(&self, eye: DVec3, section: SectionPos, block_changed: bool) -> bool {
        let view = point_of_view(eye, section);
        view != self.point_of_view || block_changed && (axis_aligned(view) || nearby(eye, section))
    }
}

#[derive(Default)]
pub struct TranslucentSorting {
    sections: HashMap<SectionPos, TranslucentQuads>,
    last_block: Option<[i64; 3]>,
    iteration: usize,
}

impl TranslucentSorting {
    pub fn compiled(&mut self, section: SectionPos, quads: Option<TranslucentQuads>) {
        match quads {
            Some(quads) => {
                self.sections.insert(section, quads);
            }
            None => {
                self.sections.remove(&section);
            }
        }
    }

    pub fn removed(&mut self, section: SectionPos) {
        self.sections.remove(&section);
    }

    pub fn resorts(
        &mut self,
        eye: DVec3,
        visible: &[(SectionPos, f32)],
    ) -> Vec<(SectionPos, SectionMesh)> {
        let mut resorts = Vec::new();
        if visible.is_empty() {
            return resorts;
        }
        let block = [eye.x, eye.y, eye.z].map(|value| value.floor() as i64);
        let block_changed = self.last_block != Some(block);
        let mut scheduled = HashSet::new();
        let mut consider = |section: SectionPos| {
            if scheduled.contains(&section) {
                return;
            }
            let Some(quads) = self.sections.get_mut(&section) else {
                return;
            };
            if quads.wants_resort(eye, section, block_changed) {
                scheduled.insert(section);
                resorts.push((section, quads.resort(eye, section)));
            }
        };
        for &(section, _) in visible {
            if nearby(eye, section) {
                consider(section);
            }
        }
        self.iteration %= visible.len();
        for _ in 0..(visible.len() / 8).max(15) {
            let index = self.iteration % visible.len();
            self.iteration += 1;
            consider(visible[index].0);
        }
        self.last_block = Some(block);
        resorts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::SectionVertex;

    fn vertex(position: [f32; 3]) -> SectionVertex {
        SectionVertex {
            position,
            uv: [0.0; 2],
            color: [255; 4],
            light: [0; 2],
            pad: [0; 2],
        }
    }

    fn panes(xs: &[f32]) -> SectionMesh {
        let mut mesh = SectionMesh::default();
        mesh.vertices.extend(
            [
                [0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
                [1.0, 1.0, 0.0],
                [1.0, 0.0, 0.0],
            ]
            .map(vertex),
        );
        mesh.indices.extend([0, 1, 2, 0, 2, 3]);
        mesh.transparent_start = Some(6);
        for &x in xs {
            let start = mesh.vertices.len() as u32;
            mesh.vertices
                .extend([[x, 0.0, 0.0], [x, 1.0, 0.0], [x, 1.0, 1.0], [x, 0.0, 1.0]].map(vertex));
            mesh.indices
                .extend([start, start + 1, start + 2, start, start + 2, start + 3]);
        }
        mesh
    }

    fn first_indices(mesh: &SectionMesh) -> Vec<u32> {
        mesh.indices[6..].chunks(6).map(|quad| quad[0]).collect()
    }

    #[test]
    fn translucent_quads_are_drawn_farthest_first() {
        let mut mesh = panes(&[1.0, 5.0, 3.0]);
        let mut quads = TranslucentQuads::of(&mesh).unwrap();
        quads.sort_mesh(&mut mesh, DVec3::new(0.0, 0.5, 0.5), (0, 0, 0));
        assert_eq!(first_indices(&mesh), vec![8, 12, 4]);
        assert_eq!(&mesh.indices[..6], &[0, 1, 2, 0, 2, 3], "opaque faces stay");
        quads.sort_mesh(&mut mesh, DVec3::new(6.0, 0.5, 0.5), (0, 0, 0));
        assert_eq!(first_indices(&mesh), vec![4, 12, 8]);
        assert_eq!(mesh.indices.len(), 24);
    }

    #[test]
    fn equally_far_quads_keep_their_built_order() {
        let mut mesh = panes(&[2.0, 2.0, 4.0]);
        let mut quads = TranslucentQuads::of(&mesh).unwrap();
        quads.sort_mesh(&mut mesh, DVec3::new(0.0, 0.5, 0.5), (0, 0, 0));
        assert_eq!(first_indices(&mesh), vec![12, 4, 8]);
    }

    #[test]
    fn the_centroid_is_halfway_between_the_first_and_third_corners() {
        let mut mesh = panes(&[]);
        let start = mesh.vertices.len() as u32;
        mesh.vertices.extend(
            [
                [0.0, 0.0, 2.0],
                [0.0, 0.0, 3.0],
                [4.0, 0.0, 3.0],
                [4.0, 0.0, 2.0],
            ]
            .map(vertex),
        );
        mesh.indices
            .extend([start, start + 2, start + 1, start, start + 3, start + 2]);
        let quads = TranslucentQuads::of(&mesh).unwrap();
        assert_eq!(quads.centroids, vec![[2.0, 0.0, 2.5]]);
    }

    #[test]
    fn a_mesh_without_translucent_faces_has_nothing_to_sort() {
        let mut mesh = panes(&[]);
        assert!(TranslucentQuads::of(&mesh).is_none());
        mesh.transparent_start = None;
        assert!(TranslucentQuads::of(&mesh).is_none());
        let mut triangle = panes(&[1.0]);
        triangle.indices.truncate(triangle.indices.len() - 3);
        assert!(
            TranslucentQuads::of(&triangle).is_none(),
            "only whole quads are sorted"
        );
    }

    #[test]
    fn the_point_of_view_is_the_side_of_the_section_the_camera_is_on() {
        assert_eq!(
            point_of_view(DVec3::new(8.0, 8.0, 8.0), (0, 0, 0)),
            [0, 0, 0]
        );
        assert_eq!(
            point_of_view(DVec3::new(-0.5, 70.0, 40.0), (0, 0, 0)),
            [-1, 1, 1]
        );
        assert_eq!(
            point_of_view(DVec3::new(-100.0, 0.0, 0.0), (0, 0, 0)),
            [-1, 0, 0]
        );
        assert!(axis_aligned([-1, 0, 1]));
        assert!(!axis_aligned([-1, 1, 1]));
        assert!(nearby(DVec3::new(40.0, 0.0, -20.0), (0, 0, 0)));
        assert!(!nearby(DVec3::new(48.0, 0.0, 0.0), (0, 0, 0)));
        assert!(nearby(DVec3::new(-32.0, 0.0, 0.0), (0, 0, 0)));
        assert!(!nearby(DVec3::new(-33.0, 0.0, 0.0), (0, 0, 0)));
    }

    #[test]
    fn resorts_follow_the_camera_like_vanilla() {
        let mesh = panes(&[1.0]);
        let mut quads = TranslucentQuads::of(&mesh).unwrap();
        let resort = quads.resort(DVec3::new(100.0, 8.0, 8.0), (0, 0, 0));
        assert!(resort.vertices.is_empty());
        assert_eq!(resort.transparent_start, Some(6));
        assert_eq!(resort.indices, vec![4, 5, 6, 4, 6, 7]);
        let far = DVec3::new(100.0, 8.0, 8.0);
        assert!(!quads.wants_resort(far, (0, 0, 0), false));
        assert!(
            quads.wants_resort(far, (0, 0, 0), true),
            "an axis-aligned view resorts each block"
        );
        assert!(
            quads.wants_resort(DVec3::new(-100.0, 8.0, 8.0), (0, 0, 0), false),
            "the other side"
        );
        quads.resort(DVec3::new(100.0, 100.0, 100.0), (0, 0, 0));
        assert!(
            !quads.wants_resort(DVec3::new(101.0, 100.0, 100.0), (0, 0, 0), true),
            "a corner view keeps its order far away"
        );
        quads.resort(DVec3::new(20.0, 20.0, 20.0), (0, 0, 0));
        assert!(
            quads.wants_resort(DVec3::new(21.0, 20.0, 20.0), (0, 0, 0), true),
            "but not nearby"
        );
    }

    #[test]
    fn nearby_sections_resort_when_the_camera_crosses_a_block() {
        let mut sorting = TranslucentSorting::default();
        let mut far = TranslucentQuads::of(&panes(&[1.0])).unwrap();
        let mut near = far.clone();
        let eye = DVec3::new(8.5, 8.5, 8.5);
        near.sort(eye, (0, 0, 0));
        far.sort(eye, (40, 40, 40));
        sorting.compiled((0, 0, 0), Some(near));
        sorting.compiled((40, 40, 40), Some(far));
        sorting.compiled((1, 0, 0), None);
        let visible = [((0, 0, 0), 1.0), ((40, 40, 40), 1.0), ((1, 0, 0), 1.0)];
        let sections = |resorts: Vec<(SectionPos, SectionMesh)>| {
            resorts
                .into_iter()
                .map(|(section, _)| section)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            sections(sorting.resorts(eye, &visible)),
            vec![(0, 0, 0)],
            "the first frame has no last block"
        );
        assert!(
            sorting
                .resorts(eye + DVec3::splat(0.25), &visible)
                .is_empty(),
            "same block, same views"
        );
        assert_eq!(
            sections(sorting.resorts(eye + DVec3::X, &visible)),
            vec![(0, 0, 0)]
        );
        sorting.removed((0, 0, 0));
        assert!(sorting.resorts(eye + DVec3::X * 2.0, &visible).is_empty());
        assert!(
            sorting
                .resorts(DVec3::new(-100.0, 8.5, 8.5), &visible)
                .is_empty(),
            "a far corner keeps its view"
        );
        assert_eq!(
            sections(sorting.resorts(DVec3::new(-100.0, 8.5, 1000.0), &visible)),
            vec![(40, 40, 40)],
            "until the camera passes its side"
        );
    }
}
