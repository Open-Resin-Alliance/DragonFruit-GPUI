//! Mesh loading — `load_mesh_file`, `load_mesh_bytes`, `load_mesh_range`.
//!
//! Ported from `src-tauri/src/mesh_repair.rs` lines 1826-2470. All three write the
//! same **DFMX** payload, the wire format the renderer parses; the format is
//! reproduced in the docs of [`encode_mesh_bodies`]. The work is entirely
//! `dragonfruit-mesh-repair`'s: parse (`io`), refine coarse faces, classify the
//! model/support split, and for an oversized STL decimate to a renderable budget.
//!
//! The one thing this module adds is [`MeshRegistry`]: a load keeps the welded
//! mesh it just built so the AO bake can name it with a 4-byte token instead of
//! being sent the soup back (36 bytes a triangle, 430 MB across a scene's models).
//! A re-weld at the bake's own epsilon was measured to produce the identical
//! vertex set and a bit-identical field, so the token is not a compromise.

use std::collections::VecDeque;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::sync::{Arc, Mutex};

use dragonfruit_mesh_core::mesh::Vec3;
use dragonfruit_mesh_core::normals::face_normals;
use dragonfruit_mesh_repair::io;
use dragonfruit_mesh_repair::repair::{classify_support_split, decimate_sections_to_budget};
use dragonfruit_mesh_repair::stl_budget::compute_triangle_budget;
use dragonfruit_mesh_repair::{IndexedMesh, RepairOptions};

use crate::BackendResult;

/// Bound on retained meshes. A load registers one and its bake takes it moments
/// later, so the cap is never reached in practice.
const MESH_REGISTRY_CAP: usize = 8;

/// An oversized binary STL above this triangle count is decimated to a preview
/// instead of being loaded whole: the renderer's representation is 72 bytes a
/// triangle *before* Three.js builds its BVH.
const STL_PREVIEW_TRIGGER_TRIANGLES: u64 = 4_000_000;
/// An ASCII STL above this size is refused: its parser has to hold the file in
/// memory.
const MAX_NATIVE_ASCII_STL_BYTES: u64 = 300_000_000;

const MESH_RESPONSE_MAGIC: &[u8; 4] = b"DFMX";
/// Version 2: each body is a zlib stream holding that body's soup. Version 1 wrote
/// the soup raw (72 bytes a triangle) and the volume was the largest single cost
/// in a scene load.
const MESH_RESPONSE_VERSION: u32 = 2;
const MESH_RESPONSE_HEADER_BYTES: usize = 32;
const MESH_RESPONSE_BODY_ENTRY_BYTES: usize = 8;
const MESH_RESPONSE_FLAG_PREVIEW: u32 = 1;
/// Positions plus normals, both 9 f32 per triangle.
const SOUP_BYTES_PER_TRIANGLE: usize = 9 * std::mem::size_of::<f32>() * 2;

/// The refined meshes a load just built, waiting for the AO bake that needs one.
///
/// Owned rather than a process-wide `static` (the shell's choice): the token is
/// returned in the payload, so nothing outside a load-and-bake pair ever looks a
/// mesh up, and per-app ownership keeps a test from inheriting another's.
#[derive(Default)]
pub struct MeshRegistry {
    entries: Mutex<VecDeque<(u32, Arc<IndexedMesh>)>>,
    next_token: Mutex<u32>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Keep `mesh` for its bake and return the token that names it.
    pub fn register(&self, mesh: IndexedMesh) -> u32 {
        let mut next = self.next_token.lock().expect("mesh registry token lock");
        if *next == 0 {
            *next = 1;
        }
        let token = *next;
        *next = next.wrapping_add(1).max(1);

        let mut entries = self.entries.lock().expect("mesh registry lock");
        entries.push_back((token, Arc::new(mesh)));
        while entries.len() > MESH_REGISTRY_CAP {
            entries.pop_front();
        }
        token
    }

    /// The mesh a token names, if it is still registered.
    pub fn get(&self, token: u32) -> Option<Arc<IndexedMesh>> {
        let entries = self.entries.lock().expect("mesh registry lock");
        entries
            .iter()
            .find(|(registered, _)| *registered == token)
            .map(|(_, mesh)| Arc::clone(mesh))
    }

    /// The mesh a bake token names, checked against the corner count the caller
    /// has.
    ///
    /// The check is what makes the token safe: a caller's geometry is the mesh it
    /// loaded, and a count that disagrees means the geometry was replaced or
    /// rewritten since — a repair, a swap, a smoothing pass — so a field computed
    /// here would be attached to a mesh it was never computed for. Refusing sends
    /// the caller back to sending the soup, which cannot go stale.
    pub fn mesh_for_bake(&self, token: u32, expected_corners: usize) -> BackendResult<Arc<IndexedMesh>> {
        let mesh =
            self.get(token)
                .ok_or_else(|| format!("mesh token {token} is no longer registered"))?;
        if expected_corners != 0 && mesh.triangles.len() * 3 != expected_corners {
            return Err(format!(
                "mesh token {token} has {} corners, the caller has {expected_corners}",
                mesh.triangles.len() * 3
            ));
        }
        Ok(mesh)
    }
}

/// `load_mesh_file`: parse a mesh file of any supported format and return its
/// bodies as a DFMX payload.
///
/// Dispatches through `io::load_mesh_bodies_from_path` (`stl`/`obj` give one body;
/// `3mf` gives one per build item, expanded through its components with transforms
/// baked), so refinement and the face normals are the ones every other import path
/// gets. Materials and units are dropped, matching the renderer's loaders.
///
/// `classify` (default true) runs the model/support classifier on a single-body
/// load and puts its report in the payload's metadata tail. A caller that will
/// repair passes `false` — that pass classifies too.
pub fn load_mesh_file(file_path: &str, classify: Option<bool>) -> BackendResult<Vec<u8>> {
    let path = Path::new(file_path);
    log::info!("[load_mesh_file] Starting native mesh load: {file_path}");

    let is_stl = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("stl"));
    if is_stl {
        if let Some(bytes) = stl_special_case_response(path, file_path, None)? {
            return Ok(bytes);
        }
    }

    let bodies = io::load_mesh_bodies_from_path(path)
        .map_err(|e| format!("Failed to load mesh '{file_path}': {e}"))?;

    let want_classify = classify.unwrap_or(true) && bodies.len() == 1;
    let (bodies, metadata_json) = if want_classify {
        classify_single_body(bodies.into_iter().next().expect("length checked above"))
    } else {
        (bodies, None)
    };

    let tri_count: usize = bodies.iter().map(|body| body.triangles.len()).sum();
    encode_mesh_bodies(
        &bodies,
        tri_count as u32,
        false,
        None,
        metadata_json.as_deref(),
        None,
    )
}

/// `load_mesh_bytes`: parse a mesh's bytes, for a source with no on-disk path — a
/// VOXL's embedded mesh chunk, or a file expanded out of a zip.
///
/// The shell took `x-mesh-classify` / `x-mesh-refine` headers because a raw-body
/// command cannot also take JSON arguments; the adapter passes them as bools.
pub fn load_mesh_bytes(bytes: &[u8], classify: bool, refine: bool) -> BackendResult<Vec<u8>> {
    log::info!(
        "[load_mesh_bytes] Starting native mesh load from {} bytes",
        bytes.len()
    );
    let started = std::time::Instant::now();
    let (bodies, metadata_json) = mesh_bodies(bytes, classify, refine)?;
    let tri_count: usize = bodies.iter().map(|body| body.triangles.len()).sum();
    let encoded = encode_mesh_bodies(
        &bodies,
        tri_count as u32,
        false,
        None,
        metadata_json.as_deref(),
        None,
    )?;
    log::info!(
        "[load_mesh_bytes] parsed, refined and encoded in {}ms",
        started.elapsed().as_millis()
    );
    Ok(encoded)
}

/// `load_mesh_range`: parse a mesh that sits at a known offset inside a file.
///
/// This is a VOXL's embedded mesh chunk: the container stores it as a packed
/// (zlib) STL at a directory-recorded offset. `compression` is the container's own
/// code — 0 none, 1 zlib — and anything else is refused rather than guessed. A
/// range that does not read exactly `length` bytes is an error: the container has
/// already been parsed by then, so a short read means the file changed under it.
///
/// When the result is a single body the welded mesh is registered for the AO bake
/// and its token rides the payload header.
pub fn load_mesh_range(
    registry: &MeshRegistry,
    source_path: &str,
    offset: u64,
    length: u64,
    compression: u16,
    classify: Option<bool>,
    refine: Option<bool>,
) -> BackendResult<Vec<u8>> {
    let started = std::time::Instant::now();
    let path = source_path.to_string();

    let (bytes, read_ms) = {
        let mut file =
            std::fs::File::open(&path).map_err(|e| format!("Failed to open '{path}': {e}"))?;
        file.seek(SeekFrom::Start(offset))
            .map_err(|e| format!("Failed to seek to {offset} in '{path}': {e}"))?;
        let mut raw = vec![0u8; length as usize];
        file.read_exact(&mut raw)
            .map_err(|e| format!("Failed to read {length} bytes at {offset} in '{path}': {e}"))?;
        let read_at = std::time::Instant::now();
        let bytes = match compression {
            0 => raw,
            1 => {
                let mut soup = Vec::with_capacity(raw.len() * 3);
                flate2::read::ZlibDecoder::new(raw.as_slice())
                    .read_to_end(&mut soup)
                    .map_err(|e| format!("Failed to inflate the mesh chunk at {offset}: {e}"))?;
                soup
            }
            other => {
                return Err(format!(
                    "Unsupported mesh chunk compression {other} at {offset} in '{path}'"
                ))
            }
        };
        (bytes, read_at.elapsed().as_millis())
    };

    let started_parse = std::time::Instant::now();
    let (bodies, metadata_json) =
        mesh_bodies(&bytes, classify.unwrap_or(true), refine.unwrap_or(true))?;

    // Keep the mesh for its AO bake. Only a single body can be named by one token.
    let token = if bodies.len() == 1 {
        Some(registry.register(bodies[0].clone()))
    } else {
        None
    };
    let tri_count: usize = bodies.iter().map(|body| body.triangles.len()).sum();
    log::info!(
        "[load_mesh_range] {} bytes at {offset} in {}ms (read+inflate), parsed in {}ms, of {}ms total{}",
        bytes.len(),
        read_ms,
        started_parse.elapsed().as_millis(),
        started.elapsed().as_millis(),
        if token.is_some() {
            ", mesh kept for the bake"
        } else {
            ""
        },
    );
    encode_mesh_bodies(
        &bodies,
        tri_count as u32,
        false,
        None,
        metadata_json.as_deref(),
        token,
    )
}

/// Parse a file's bytes, refine its coarse faces when asked, and classify when
/// asked.
///
/// `refine` is false for the mesh a VOXL stores. That payload is the scene
/// geometry the app already refined, and refinement is not idempotent: on the
/// reported scene it turned 81,282 triangles into 126,386 on the next load, so a
/// `classification` written for the first count could never address the second,
/// every load re-ran the classifier (which reorders the soup), and a save/load
/// cycle grew the mesh by half. Refining what is already refined is the one thing
/// a reader must not do.
fn mesh_bodies(
    bytes: &[u8],
    classify: bool,
    refine: bool,
) -> BackendResult<(Vec<IndexedMesh>, Option<String>)> {
    let mesh = io::stl::parse_bytes(bytes)
        .map_err(|e| format!("Failed to parse mesh bytes: {e}"))?;
    let mesh = if refine { io::refine_coarse_faces(mesh) } else { mesh };
    Ok(if classify {
        classify_single_body(mesh)
    } else {
        (vec![mesh], None)
    })
}

/// Classify one body: section-ordered mesh plus the report as JSON.
///
/// Both loaders share this, so a file and a byte payload produce the same report
/// for the same mesh, and the report rides the DFMX metadata tail either way.
fn classify_single_body(mesh: IndexedMesh) -> (Vec<IndexedMesh>, Option<String>) {
    let started = std::time::Instant::now();
    let outcome = classify_support_split(mesh, &RepairOptions::default());
    log::info!(
        "[load_mesh] classified in {}ms (model section {:?} triangles)",
        started.elapsed().as_millis(),
        outcome.report.model_triangle_count,
    );
    for step in &outcome.report.steps {
        log::info!("[load_mesh]   step {} in {:.1}ms", step.name, step.elapsed_ms);
    }
    log::info!(
        "[load_mesh]   native report total {:.1}ms",
        outcome.report.total_ms
    );
    (vec![outcome.mesh], serde_json::to_string(&outcome.report).ok())
}

/// The STL-only step of a native load.
///
/// A binary STL announces its triangle count in its 84-byte header, so a file too
/// large to render can be found — and decimated — before the repair loader reads
/// and indexes the whole thing. This is also where an oversized ASCII STL is
/// refused, because its parser has to hold the file in memory.
///
/// `Ok(Some(bytes))` when the preview path produced a DFMX payload, `Ok(None)`
/// when the file is ordinary and the caller should load it normally.
fn stl_special_case_response(
    path: &Path,
    file_path: &str,
    forced_model_triangle_count: Option<u32>,
) -> BackendResult<Option<Vec<u8>>> {
    let file_size = std::fs::metadata(path)
        .map_err(|e| format!("Failed to inspect STL '{file_path}': {e}"))?
        .len();
    let mut header = [0u8; 84];
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open STL '{file_path}': {e}"))?;
    let header_len = file
        .read(&mut header)
        .map_err(|e| format!("Failed to read STL header '{file_path}': {e}"))?;

    if header_len == header.len() {
        let triangle_count = u32::from_le_bytes(header[80..84].try_into().unwrap()) as u64;
        let expected_binary_size = 84u64.saturating_add(triangle_count.saturating_mul(50));
        let force_decimate = forced_model_triangle_count.is_some();
        if expected_binary_size == file_size
            && (triangle_count > STL_PREVIEW_TRIGGER_TRIANGLES || force_decimate)
        {
            let (bbox_min, bbox_max) = binary_stl_bounds(path, triangle_count as u32)?;
            let bbox_diagonal_mm = bbox_max.sub(bbox_min).length() as f64;
            drop(file);

            let budget = compute_triangle_budget(triangle_count as usize, bbox_diagonal_mm, None);

            // Through the dispatcher, not `io::stl::load`, so the model gets the
            // same coarse-face refinement every other import path applies.
            let mesh = io::load_mesh_from_path(path)
                .map_err(|e| format!("Failed to load STL '{file_path}': {e}"))?;

            let (mesh_to_decimate, model_tri_count) = match forced_model_triangle_count {
                Some(provided) => {
                    let count = provided.min(mesh.triangles.len() as u32) as usize;
                    (mesh, count)
                }
                None => {
                    let classify_outcome = classify_support_split(mesh, &RepairOptions::default());
                    let count = classify_outcome
                        .report
                        .model_triangle_count
                        .unwrap_or(classify_outcome.mesh.triangles.len());
                    (classify_outcome.mesh, count)
                }
            };

            let outcome = decimate_sections_to_budget(mesh_to_decimate, model_tri_count, &budget);
            let preview = outcome.mesh;

            log::info!(
                "[load_mesh_file] Preview complete: {} -> {} triangles (budget: {}, error: {:.4})",
                triangle_count,
                preview.triangles.len(),
                budget.budget_tris,
                outcome.achieved_error
            );
            return encode_mesh_bodies(
                std::slice::from_ref(&preview),
                triangle_count as u32,
                true,
                Some(outcome.model_triangle_count as u32),
                None,
                None,
            )
            .map(Some);
        }
    }

    if file_size > MAX_NATIVE_ASCII_STL_BYTES && header.starts_with(b"solid") {
        return Err(format!(
            "ASCII STL is too large for the current renderer ({:.2} GB on disk; limit {:.2} GB). Decimate or convert it before importing.",
            file_size as f64 / 1_000_000_000.0,
            MAX_NATIVE_ASCII_STL_BYTES as f64 / 1_000_000_000.0,
        ));
    }
    Ok(None)
}

fn read_binary_stl_vertex(record: &[u8; 50], offset: usize) -> Vec3 {
    let read_f32 = |at: usize| f32::from_le_bytes(record[at..at + 4].try_into().unwrap());
    Vec3::new(read_f32(offset), read_f32(offset + 4), read_f32(offset + 8))
}

/// The bounding box of a binary STL, read record by record.
///
/// Needed *before* the loader so the decimation budget can be sized from the
/// model's own diagonal.
fn binary_stl_bounds(path: &Path, triangle_count: u32) -> BackendResult<(Vec3, Vec3)> {
    let file = std::fs::File::open(path)
        .map_err(|e| format!("Failed to open STL preview source: {e}"))?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    reader
        .seek(SeekFrom::Start(84))
        .map_err(|e| format!("Failed seeking STL: {e}"))?;
    let mut record = [0u8; 50];
    let mut min = Vec3::new(f32::INFINITY, f32::INFINITY, f32::INFINITY);
    let mut max = Vec3::new(f32::NEG_INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY);
    for triangle_index in 0..triangle_count {
        reader.read_exact(&mut record).map_err(|e| {
            format!("Truncated binary STL at triangle {triangle_index}: {e}")
        })?;
        for offset in [12, 24, 36] {
            let vertex = read_binary_stl_vertex(&record, offset);
            min = min.min(vertex);
            max = max.max(vertex);
        }
    }
    Ok((min, max))
}

/// DragonFruit Mesh Transfer (DFMX) binary payload. Format version 2.
///
/// Header (32 bytes):
/// ```text
///   0 ..  3 : ASCII magic "DFMX"
///   4 ..  7 : version u32 LE (2)
///   8 .. 11 : flags u32 LE (bit 0: IS_PREVIEW)
///  12 .. 15 : body count u32 LE
///  16 .. 19 : original input triangle count u32 LE
///  20 .. 23 : model section triangle count u32 LE (0 when absent)
///  24 .. 27 : metadata JSON byte length u32 LE (0 when absent)
///  28 .. 31 : mesh token u32 LE (0 when the loader kept nothing)
/// ```
/// Body table: `bodyCount` entries of 8 bytes — triangle count u32 LE, length of
/// this body's packed stream u32 LE — in payload order.
///
/// Payload: each body in table order, as a zlib stream that unpacks to its soup
/// (positions then normals, 9 f32 per triangle each). The metadata JSON, when
/// present, is the last `metadataLength` bytes — a reader that does not know the
/// key reads the bodies and ignores the tail.
fn encode_mesh_bodies(
    bodies: &[IndexedMesh],
    original_triangle_count: u32,
    is_preview: bool,
    model_triangle_count: Option<u32>,
    metadata_json: Option<&str>,
    mesh_token: Option<u32>,
) -> BackendResult<Vec<u8>> {
    let total_triangles: usize = bodies.iter().map(|body| body.triangles.len()).sum();
    let metadata_len = metadata_json.map_or(0, str::len);
    let packed: Vec<Vec<u8>> = bodies
        .iter()
        .map(encode_body_deflated)
        .collect::<BackendResult<_, _>>()?;
    let packed_len: usize = packed.iter().map(Vec::len).sum();
    let response_len = MESH_RESPONSE_HEADER_BYTES
        .checked_add(bodies.len() * MESH_RESPONSE_BODY_ENTRY_BYTES)
        .and_then(|size| size.checked_add(packed_len))
        .and_then(|size| size.checked_add(metadata_len))
        .ok_or_else(|| "DFMX response size overflow".to_string())?;

    let mut result = Vec::new();
    result.try_reserve_exact(response_len).map_err(|_| {
        format!(
            "Not enough memory for the mesh response ({:.2} GB)",
            response_len as f64 / 1_000_000_000.0
        )
    })?;
    result.extend_from_slice(MESH_RESPONSE_MAGIC);
    result.extend_from_slice(&MESH_RESPONSE_VERSION.to_le_bytes());
    result.extend_from_slice(
        &(if is_preview {
            MESH_RESPONSE_FLAG_PREVIEW
        } else {
            0
        })
        .to_le_bytes(),
    );
    result.extend_from_slice(&(bodies.len() as u32).to_le_bytes());
    result.extend_from_slice(&original_triangle_count.to_le_bytes());
    result.extend_from_slice(&model_triangle_count.unwrap_or(0).to_le_bytes());
    result.extend_from_slice(&(metadata_len as u32).to_le_bytes());
    result.extend_from_slice(&mesh_token.unwrap_or(0).to_le_bytes());
    for (body, bytes) in bodies.iter().zip(&packed) {
        result.extend_from_slice(&(body.triangles.len() as u32).to_le_bytes());
        result.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    }
    for bytes in &packed {
        result.extend_from_slice(bytes);
    }
    if let Some(json) = metadata_json {
        result.extend_from_slice(json.as_bytes());
    }
    debug_assert_eq!(result.len(), response_len);

    log::info!(
        "[load_mesh] {} bodies, {} triangles, {:.1} MB payload ({:.1}x deflated), {} B metadata",
        bodies.len(),
        total_triangles,
        (response_len - MESH_RESPONSE_HEADER_BYTES) as f64 / 1_000_000.0,
        total_triangles as f64 * SOUP_BYTES_PER_TRIANGLE as f64 / packed_len.max(1) as f64,
        metadata_len,
    );
    Ok(result)
}

/// One body's soup, zlib-packed at the fastest level.
///
/// Level 1 rather than the default: the encoder runs per model on the import's
/// critical path, and on this data the difference is 2.7x against 3.6x for six
/// times the CPU.
fn encode_body_deflated(body: &IndexedMesh) -> BackendResult<Vec<u8>> {
    use std::io::Write;

    let mut raw = vec![0u8; body.triangles.len() * SOUP_BYTES_PER_TRIANGLE];
    write_soup(body, &mut raw);
    let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder
        .write_all(&raw)
        .map_err(|e| format!("DFMX body compression failed: {e}"))?;
    encoder
        .finish()
        .map_err(|e| format!("DFMX body compression failed: {e}"))
}

/// Write a mesh's soup — positions (9 f32 per triangle), then one normal per
/// corner — into `out`, which must be exactly `triangleCount * 72` bytes.
///
/// Every corner carries its own triangle's geometric normal, so the viewport
/// shades the facets the file describes. The geometry a repair pass hands back
/// gets the same normals from the renderer's per-face recompute, so repairing a
/// model does not change how it looks.
fn write_soup(mesh: &IndexedMesh, out: &mut [u8]) {
    use rayon::prelude::*;

    let positions_len = mesh.triangles.len() * 9 * std::mem::size_of::<f32>();
    let (position_output, normal_output) = out.split_at_mut(positions_len);
    position_output
        .par_chunks_mut(9 * std::mem::size_of::<f32>())
        .zip(mesh.triangles.par_iter())
        .for_each(|(output, triangle)| {
            let vertices = [
                mesh.positions[triangle[0] as usize],
                mesh.positions[triangle[1] as usize],
                mesh.positions[triangle[2] as usize],
            ];
            for (vertex_output, vertex) in output.chunks_exact_mut(12).zip(vertices) {
                vertex_output[0..4].copy_from_slice(&vertex.x.to_le_bytes());
                vertex_output[4..8].copy_from_slice(&vertex.y.to_le_bytes());
                vertex_output[8..12].copy_from_slice(&vertex.z.to_le_bytes());
            }
        });

    let normals = face_normals(mesh);
    normal_output
        .par_chunks_mut(9 * std::mem::size_of::<f32>())
        .zip(normals.par_iter())
        .for_each(|(output, normal)| {
            for normal_output in output.chunks_exact_mut(12) {
                normal_output[0..4].copy_from_slice(&normal.x.to_le_bytes());
                normal_output[4..8].copy_from_slice(&normal.y.to_le_bytes());
                normal_output[8..12].copy_from_slice(&normal.z.to_le_bytes());
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read as _;

    fn temp_dir(label: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("df-mesh-io-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("dir");
        dir
    }

    /// A binary STL holding one triangle.
    fn one_triangle_stl() -> Vec<u8> {
        let mut bytes = vec![0u8; 80];
        bytes.extend_from_slice(&1u32.to_le_bytes());
        // Face normal, then three vertices, each three little-endian f32s.
        for _ in 0..3 {
            bytes.extend_from_slice(&0.0f32.to_le_bytes());
        }
        for vertex in [[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]] {
            for value in vertex {
                bytes.extend_from_slice(&value.to_le_bytes());
            }
        }
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes
    }

    fn parse_header(bytes: &[u8]) -> (u32, u32, u32, u32, u32, u32) {
        assert_eq!(&bytes[0..4], &MESH_RESPONSE_MAGIC[..]);
        let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
        let flags = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
        let body_count = u32::from_le_bytes(bytes[12..16].try_into().unwrap());
        let original = u32::from_le_bytes(bytes[16..20].try_into().unwrap());
        let metadata_len = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let token = u32::from_le_bytes(bytes[28..32].try_into().unwrap());
        (version, flags, body_count, original, metadata_len, token)
    }

    fn first_body_soup(bytes: &[u8]) -> Vec<u8> {
        let tri_count = u32::from_le_bytes(
            bytes[MESH_RESPONSE_HEADER_BYTES..MESH_RESPONSE_HEADER_BYTES + 4]
                .try_into()
                .unwrap(),
        ) as usize;
        let packed_len = u32::from_le_bytes(
            bytes[MESH_RESPONSE_HEADER_BYTES + 4..MESH_RESPONSE_HEADER_BYTES + 8]
                .try_into()
                .unwrap(),
        ) as usize;
        let start = MESH_RESPONSE_HEADER_BYTES + MESH_RESPONSE_BODY_ENTRY_BYTES;
        let mut soup = Vec::new();
        flate2::read::ZlibDecoder::new(&bytes[start..start + packed_len])
            .read_to_end(&mut soup)
            .expect("inflate");
        assert_eq!(soup.len(), tri_count * SOUP_BYTES_PER_TRIANGLE);
        soup
    }

    #[test]
    fn load_mesh_file_writes_a_version_two_payload() {
        let dir = temp_dir("load-file");
        let file = dir.join("one.stl");
        std::fs::write(&file, one_triangle_stl()).expect("write stl");

        let payload = load_mesh_file(&file.to_string_lossy(), Some(false)).expect("load");
        let (version, flags, bodies, original, metadata_len, token) = parse_header(&payload);
        assert_eq!(version, MESH_RESPONSE_VERSION);
        assert_eq!(flags, 0);
        assert_eq!(bodies, 1);
        assert_eq!(original, 1);
        assert_eq!(metadata_len, 0);
        assert_eq!(token, 0);
        assert_eq!(first_body_soup(&payload).len(), SOUP_BYTES_PER_TRIANGLE);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_mesh_bytes_refuses_bytes_that_are_not_a_mesh() {
        // The byte-source loader must refuse rather than guess.
        let err = load_mesh_bytes(b"not a mesh at all", false, false).expect_err("refuse");
        assert!(err.contains("parse mesh bytes"), "{err}");
    }

    #[test]
    fn unknown_chunk_compression_is_refused() {
        let dir = temp_dir("compression");
        let file = dir.join("one.stl");
        std::fs::write(&file, one_triangle_stl()).expect("write");
        let registry = MeshRegistry::new();
        let err = load_mesh_range(&registry, &file.to_string_lossy(), 0, 84, 7, None, None)
            .expect_err("refuse");
        assert!(err.contains("Unsupported mesh chunk compression 7"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_mesh_range_registers_the_single_body() {
        let dir = temp_dir("range");
        let file = dir.join("one.stl");
        let bytes = one_triangle_stl();
        std::fs::write(&file, &bytes).expect("write");

        let registry = MeshRegistry::new();
        let payload = load_mesh_range(
            &registry,
            &file.to_string_lossy(),
            0,
            bytes.len() as u64,
            0,
            Some(false),
            Some(false),
        )
        .expect("range");
        let (_, _, bodies, _, _, token) = parse_header(&payload);
        assert_eq!(bodies, 1);
        assert_ne!(token, 0);

        let mesh = registry.get(token).expect("registered");
        assert_eq!(mesh.triangles.len(), 1);
        // A corner count that disagrees means the caller's geometry moved on.
        registry.mesh_for_bake(token, 3).expect("matching corners");
        let err = registry
            .mesh_for_bake(token, 9)
            .expect_err("stale corner count");
        assert!(err.contains("the caller has 9"), "{err}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_registry_is_bounded() {
        let registry = MeshRegistry::new();
        let mesh = IndexedMesh {
            positions: vec![Vec3::new(0.0, 0.0, 0.0)],
            triangles: Vec::new(),
        };
        let mut first = 0;
        for index in 0..(MESH_REGISTRY_CAP + 2) {
            let token = registry.register(mesh.clone());
            if index == 0 {
                first = token;
            }
        }
        assert!(registry.get(first).is_none(), "the oldest entry is dropped");
    }
}
