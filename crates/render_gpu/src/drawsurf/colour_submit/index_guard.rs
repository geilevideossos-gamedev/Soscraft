use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use bevy::render::render_resource::IndexFormat;
use render_frame::{CODE_MESH_INDEX_CAP, CODE_MESH_VERT_CAP};

use super::{ExactColourGeometry, ExactTessBind, GpuSubmitRefusal};

const LOG_FIRST: u32 = 8;
const LOG_EVERY: u32 = 600;

static SKIPPED_DRAWS: AtomicU32 = AtomicU32::new(0);
static CODE_MESH_OVER_CAP_REPORTED: AtomicBool = AtomicBool::new(false);

pub(super) fn range_within(start: u32, count: u32, len: usize) -> bool {
    u64::from(start) + u64::from(count) <= len as u64
}

pub(super) fn index_stride(format: IndexFormat) -> u64 {
    match format {
        IndexFormat::Uint16 => 2,
        IndexFormat::Uint32 => 4,
    }
}

pub(super) fn index_range_fits(
    start: u32,
    count: u32,
    buffer_bytes: u64,
    format: IndexFormat,
) -> bool {
    (u64::from(start) + u64::from(count)) * index_stride(format) <= buffer_bytes
}

pub(super) fn resident_model_index_len(
    gpu_len: usize,
    cpu_len: usize,
    gpu_topology: u64,
    cpu_topology: u64,
) -> usize {
    if gpu_topology == cpu_topology {
        gpu_len.min(cpu_len)
    } else {
        0
    }
}

fn uploaded_index_len(geometry: &ExactColourGeometry, tess: ExactTessBind) -> Option<usize> {
    match tess {
        ExactTessBind::XModel => Some(geometry.xmodel.index.len()),
        ExactTessBind::ParticleCloud => Some(geometry.particle_cloud.index.len()),
        ExactTessBind::CodeMesh => Some(geometry.fx_index_count.min(CODE_MESH_INDEX_CAP as usize)),
        ExactTessBind::MarkMesh => Some(geometry.mark_mesh.index.len()),
        ExactTessBind::Glass => Some(geometry.glass_mesh.index.len()),
        ExactTessBind::World
        | ExactTessBind::Smodel
        | ExactTessBind::SmodelCached
        | ExactTessBind::SmodelSkinned => None,
    }
}

pub(super) fn beyond_upload(
    geometry: &ExactColourGeometry,
    tess: ExactTessBind,
    start: u32,
    count: u32,
) -> Option<GpuSubmitRefusal> {
    let len = uploaded_index_len(geometry, tess)?;
    if range_within(start, count, len) {
        return None;
    }
    Some(GpuSubmitRefusal::IndexRangeBeyondUpload {
        tess,
        end: u64::from(start) + u64::from(count),
        len: len as u64,
    })
}

pub(super) fn code_mesh_over_cap(vertices: usize, indices: usize) -> bool {
    let over = vertices > CODE_MESH_VERT_CAP as usize || indices > CODE_MESH_INDEX_CAP as usize;
    if over && !CODE_MESH_OVER_CAP_REPORTED.swap(true, Ordering::Relaxed) {
        diag::warn!(
            World,
            "render: the code mesh holds {vertices} vertices and {indices} indices, past its {CODE_MESH_VERT_CAP}-vertex and {CODE_MESH_INDEX_CAP}-index buffer, so its draws are skipped"
        );
    }
    over
}

fn logs_skip(skipped: u32) -> bool {
    skipped < LOG_FIRST || skipped.is_multiple_of(LOG_EVERY)
}

pub(super) fn report_skipped_draw(
    label: &str,
    tess: ExactTessBind,
    start: u32,
    count: u32,
    limit: u64,
) {
    let skipped = SKIPPED_DRAWS.fetch_add(1, Ordering::Relaxed);
    if logs_skip(skipped) {
        diag::warn!(
            World,
            "render: {label} skipped a {tess:?} draw at {start}+{count}, past its {limit}-index buffer ({} skipped so far)",
            skipped.saturating_add(1)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_crash_range_does_not_fit_a_seven_granule_stream() {
        assert!(!index_range_fits(0, 138_678, 458_752, IndexFormat::Uint32));
        assert!(index_range_fits(0, 114_688, 458_752, IndexFormat::Uint32));
    }

    #[test]
    fn u16_streams_hold_twice_the_indices() {
        assert!(index_range_fits(0, 229_376, 458_752, IndexFormat::Uint16));
        assert!(!index_range_fits(0, 229_377, 458_752, IndexFormat::Uint16));
    }

    #[test]
    fn an_overflowing_range_never_fits() {
        assert!(!index_range_fits(u32::MAX, 3, 458_752, IndexFormat::Uint32));
        assert!(!range_within(u32::MAX, 3, u32::MAX as usize));
    }

    #[test]
    fn sun_spans_use_the_gpu_length_when_topology_matches() {
        assert_eq!(resident_model_index_len(114_688, 138_678, 7, 7), 114_688);
        assert_eq!(resident_model_index_len(138_678, 114_688, 7, 7), 114_688);
    }

    #[test]
    fn sun_spans_are_refused_while_the_topology_is_not_resident() {
        assert_eq!(resident_model_index_len(114_688, 138_678, 7, 8), 0);
    }

    #[test]
    fn range_within_counts_the_end() {
        assert!(range_within(0, 3, 3));
        assert!(!range_within(1, 3, 3));
    }

    #[test]
    fn a_range_past_the_uploaded_xmodel_stream_is_refused() {
        let mut geometry = ExactColourGeometry::default();
        geometry.xmodel.index.set_len(114_688);
        assert_eq!(
            beyond_upload(&geometry, ExactTessBind::XModel, 0, 138_678),
            Some(GpuSubmitRefusal::IndexRangeBeyondUpload {
                tess: ExactTessBind::XModel,
                end: 138_678,
                len: 114_688,
            })
        );
        assert_eq!(
            beyond_upload(&geometry, ExactTessBind::XModel, 0, 114_688),
            None
        );
    }

    #[test]
    fn the_code_mesh_is_checked_against_its_ring_cap() {
        let geometry = ExactColourGeometry {
            fx_index_count: CODE_MESH_INDEX_CAP as usize + 300,
            ..ExactColourGeometry::default()
        };
        let cap = CODE_MESH_INDEX_CAP;
        assert_eq!(
            beyond_upload(&geometry, ExactTessBind::CodeMesh, 0, cap),
            None
        );
        assert!(beyond_upload(&geometry, ExactTessBind::CodeMesh, 0, cap + 3).is_some());
    }

    #[test]
    fn static_streams_are_left_to_their_own_checks() {
        let geometry = ExactColourGeometry::default();
        assert_eq!(beyond_upload(&geometry, ExactTessBind::World, 0, 3), None);
        assert_eq!(
            beyond_upload(&geometry, ExactTessBind::SmodelCached, 0, 3),
            None
        );
    }

    #[test]
    fn a_code_mesh_past_its_cap_is_reported() {
        assert!(!code_mesh_over_cap(
            CODE_MESH_VERT_CAP as usize,
            CODE_MESH_INDEX_CAP as usize
        ));
        assert!(code_mesh_over_cap(0, CODE_MESH_INDEX_CAP as usize + 1));
        assert!(code_mesh_over_cap(CODE_MESH_VERT_CAP as usize + 1, 0));
    }

    #[test]
    fn skipped_draws_log_the_first_eight_then_every_six_hundredth() {
        assert!((0..8).all(logs_skip));
        assert!(!logs_skip(8));
        assert!(!logs_skip(599));
        assert!(logs_skip(600));
        assert!(!logs_skip(601));
        assert!(logs_skip(1200));
    }
}
