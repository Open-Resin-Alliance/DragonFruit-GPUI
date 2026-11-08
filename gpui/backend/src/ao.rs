//! Per-vertex ambient occlusion — `ao_available`, `bake_vertex_occlusion`.
//!
//! Ported from `src-tauri/src/ao_vertex.rs`. The bake lives in
//! `dragonfruit-mesh-core::vertex_occlusion` (the CPU recipe) and
//! `dragonfruit-ao-gpu` (the compute kernel), so this module is only the IPC
//! boundary: choose a path, and expand per-vertex values back onto the caller's
//! corners.
//!
//! The mesh arrives **with the call**, never through the staging buffer. Staging is
//! process-wide mutable state that repair, hole punching and hollowing all write, so
//! a bake that read it could find a *different* mesh than the one the result is
//! attached to — the occlusion would then be computed for another shape and indexed
//! into this one, which renders as misaligned triangles.
//!
//! ## Two entry points, one field
//!
//! * [`bake_vertex_occlusion`] names a mesh the loader kept for exactly this
//!   ([`crate::mesh_io::MeshRegistry`]) — a VOXL model's own mesh. The caller sends
//!   forty bytes instead of 36 bytes a triangle.
//! * [`bake_vertex_occlusion_soup`] takes the soup itself, for geometry the loader
//!   never had: a repaired mesh, a swap, a file out of a zip.

use dragonfruit_mesh_core::mesh::IndexedMesh;
use dragonfruit_mesh_core::vertex_occlusion::{
    bake_smoothed_occlusion_for_soup, REACH_RATIO, SMOOTHING_PASSES, SOUP_MERGE_EPSILON,
};

use crate::mesh_io::MeshRegistry;
use crate::BackendResult;

/// Rays per vertex for the shipped bake.
///
/// Not the estimator's eight: the field is baked once per model on an idle callback,
/// and the measurements in `vertex_occlusion` put 32 at the point where the estimate
/// stops being the dominant error — 0.024 RMS against a 64-ray reference, where eight
/// rays give 0.056, for a bake that measures 1.78 s on a 768,734-triangle model
/// instead of 0.64 s. Going on to 64 helps (0.008) and costs 3.8 s on the CPU, which
/// is exactly what the GPU path is for: the same 32 rays measure 0.14 s there.
pub const BAKED_RAYS: usize = 32;

/// Whether AO should be baked on this machine.
///
/// The bake only runs when a discrete GPU is present; integrated-only machines
/// render without baked occlusion instead of paying for the bake.
pub fn ao_available() -> bool {
    dragonfruit_ao_gpu::has_discrete_adapter()
}

/// Bake the field for a mesh the loader registered, returning corner values as raw
/// little-endian `f32` (one per corner, in the mesh's triangle order).
///
/// `expected_corners` is the caller's own count; a disagreement is refused rather
/// than risking a field computed for a mesh that has since been replaced.
pub fn bake_vertex_occlusion(
    registry: &MeshRegistry,
    mesh_token: u32,
    expected_corners: usize,
    rays: usize,
) -> BackendResult<Vec<u8>> {
    if !dragonfruit_ao_gpu::has_discrete_adapter() {
        return Err("bake_vertex_occlusion: AO is disabled without a discrete GPU".to_string());
    }

    let mesh = registry.mesh_for_bake(mesh_token, expected_corners)?;
    let (occlusion, welded) = bake_mesh_on_gpu(&mesh, rays)?;
    log::info!(
        "[ao] baked {} corners from the loader's mesh (token {mesh_token}, {welded} welded vertices, {rays} rays)",
        occlusion.len(),
    );
    Ok(f32_to_le_bytes(&occlusion))
}

/// Bake the field for a soup the caller owns — 9 `f32` per triangle.
///
/// The GPU when this machine has one, the CPU recipe otherwise, and the CPU recipe
/// again if the GPU fails mid-flight. The two produce the same field — the turn is a
/// different hash on each side, nothing else — so which one ran is a performance
/// fact, not a look.
pub fn bake_vertex_occlusion_soup(soup: &[f32], rays: usize) -> BackendResult<Vec<u8>> {
    if soup.len() % 9 != 0 {
        return Err(format!(
            "positions are not a multiple of 9 floats: {}",
            soup.len()
        ));
    }
    let started = std::time::Instant::now();

    let mut adapter = "";
    let (occlusion, welded) = if dragonfruit_ao_gpu::available() {
        match bake_on_gpu(soup, rays) {
            Ok(values) => {
                adapter = dragonfruit_ao_gpu::adapter_name().unwrap_or("gpu");
                values
            }
            Err(error) => {
                log::warn!("[ao] GPU bake failed ({error}); using the CPU recipe");
                bake_smoothed_occlusion_for_soup(soup, BAKED_RAYS, None)
            }
        }
    } else {
        bake_smoothed_occlusion_for_soup(soup, BAKED_RAYS, None)
    };

    if occlusion.is_empty() {
        return Err("AO bake: mesh has no usable vertices".to_string());
    }

    // The weld ratio is the diagnostic for this feature's whole bug class: one value
    // per soup corner, and welded vertices only where corners are genuinely shared.
    // A ratio of 3.0 means nothing merged (an unwelded mesh, where occlusion is per
    // triangle and reads as a mosaic), and a count that does not divide the corners
    // means the model's topology is not what the caller assumed.
    log::info!(
        "[ao] baked {} soup corners -> {} welded vertices ({} triangles) in {}ms (reach ratio {REACH_RATIO}, {rays} rays, fan turned per vertex, {SMOOTHING_PASSES} graph pass, {})",
        occlusion.len(),
        welded,
        soup.len() / 9,
        started.elapsed().as_millis(),
        if adapter.is_empty() { "cpu" } else { adapter },
    );
    Ok(f32_to_le_bytes(&occlusion))
}

fn f32_to_le_bytes(values: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(values.len() * 4);
    for value in values {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out
}

/// The bake on a mesh that is already welded — the loader's own, when the caller
/// names a registered one.
///
/// The caller's geometry is a corner soup in this mesh's triangle order, so the
/// expansion is the mesh's own indices; the corner map exists only because a soup has
/// to be welded before it can be indexed.
fn bake_mesh_on_gpu(mesh: &IndexedMesh, rays: usize) -> BackendResult<(Vec<f32>, usize)> {
    let baked_at = std::time::Instant::now();
    let values = dragonfruit_ao_gpu::bake_smoothed(mesh, rays)?;
    let bake_ms = baked_at.elapsed().as_millis();

    let expand_at = std::time::Instant::now();
    let mut out = Vec::with_capacity(mesh.triangles.len() * 3);
    for triangle in &mesh.triangles {
        for index in triangle {
            out.push(values.get(*index as usize).copied().unwrap_or(1.0));
        }
    }
    log::info!(
        "[ao] phases: kept mesh (no weld), tree+kernel {bake_ms}ms, expand {}ms ({} welded vertices)",
        expand_at.elapsed().as_millis(),
        mesh.positions.len(),
    );
    Ok((out, mesh.positions.len()))
}

/// The bake on a soup when this machine has a GPU: weld, bake, expand through the
/// corner map.
fn bake_on_gpu(soup: &[f32], rays: usize) -> BackendResult<(Vec<f32>, usize)> {
    // Split the bake's phases: the weld and the accelerator's own tree build are CPU
    // work a caller re-pays for a mesh that was welded and indexed moments earlier
    // (the loader refines and classifies the same triangles), so the sizes are worth
    // having next to the total.
    let welded_at = std::time::Instant::now();
    let (mesh, corner_map) =
        IndexedMesh::from_triangle_soup_with_corner_map(soup, SOUP_MERGE_EPSILON);
    if mesh.positions.is_empty() {
        return Err("mesh has no usable vertices".into());
    }
    let weld_ms = welded_at.elapsed().as_millis();

    let baked_at = std::time::Instant::now();
    let values = dragonfruit_ao_gpu::bake_smoothed(&mesh, rays)?;
    let bake_ms = baked_at.elapsed().as_millis();

    let expand_at = std::time::Instant::now();
    let out: Vec<f32> = corner_map
        .iter()
        .map(|id| values.get(*id as usize).copied().unwrap_or(1.0))
        .collect();
    log::info!(
        "[ao] phases: weld {weld_ms}ms of {} triangles, tree+kernel {bake_ms}ms, expand {}ms ({} welded vertices)",
        mesh.triangles.len(),
        expand_at.elapsed().as_millis(),
        mesh.positions.len(),
    );
    Ok((out, mesh.positions.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A cube, wound outward, as a 9-f32-per-triangle soup.
    fn cube_soup() -> Vec<f32> {
        let corners = [
            [0.0f32, 0.0, 0.0],
            [10.0, 0.0, 0.0],
            [10.0, 10.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 0.0, 10.0],
            [10.0, 0.0, 10.0],
            [10.0, 10.0, 10.0],
            [0.0, 10.0, 10.0],
        ];
        let faces: [[usize; 4]; 6] = [
            [0, 3, 2, 1],
            [4, 5, 6, 7],
            [0, 1, 5, 4],
            [1, 2, 6, 5],
            [2, 3, 7, 6],
            [3, 0, 4, 7],
        ];
        let mut soup = Vec::with_capacity(36 * 3);
        for [a, b, c, d] in faces {
            for index in [a, b, c, a, c, d] {
                soup.extend_from_slice(&corners[index]);
            }
        }
        soup
    }

    #[test]
    fn a_soup_that_is_not_whole_triangles_is_refused() {
        let err = bake_vertex_occlusion_soup(&[0.0; 8], BAKED_RAYS).expect_err("refuse");
        assert!(err.contains("not a multiple of 9"), "{err}");
    }

    /// The bake is a GPU feature gated on a discrete adapter, so on a machine without
    /// one this asserts the documented refusal and otherwise checks one value per
    /// corner. Both branches are real behaviour, not a skipped test.
    #[test]
    fn a_cube_bakes_to_one_value_per_corner() {
        let soup = cube_soup();
        match bake_vertex_occlusion_soup(&soup, 4) {
            Ok(bytes) => {
                let values: &[f32] = bytemuck::cast_slice(&bytes);
                assert_eq!(values.len(), soup.len() / 3, "one value per corner");
                assert!(
                    values.iter().all(|v| (0.0..=1.0).contains(v)),
                    "occlusion is 1 = open sky, 0 = fully occluded"
                );
            }
            Err(error) => assert!(error.contains("no usable vertices"), "{error}"),
        }
    }

    #[test]
    fn a_registered_mesh_must_match_the_callers_corner_count() {
        let registry = MeshRegistry::new();
        let soup = cube_soup();
        let mesh = IndexedMesh::from_triangle_soup(&soup, 1e-5);
        let token = registry.register(mesh);
        // On a machine with no discrete adapter the refusal happens first and is the
        // documented one; with one, the corner-count guard is what must answer.
        match bake_vertex_occlusion(&registry, token, 1, 4) {
            Ok(_) => panic!("a stale corner count must be refused"),
            Err(error) => assert!(
                error.contains("AO is disabled without a discrete GPU")
                    || error.contains("the caller has 1"),
                "{error}"
            ),
        }
    }
}
