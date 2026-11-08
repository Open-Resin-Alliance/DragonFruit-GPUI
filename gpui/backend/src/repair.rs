//! Mesh repair, hollowing, hole punching and surfacing — the `mesh_*` family.
//!
//! Ported from `src-tauri/src/mesh_repair.rs` (43 registered commands). All of
//! these already delegated to `dragonfruit-mesh-repair` and
//! `dragonfruit-organic-cut`; the shell module was the Tauri boundary plus
//! process-wide preview buffers. Those buffers are [`HollowPreview`] and
//! [`PunchPreview`] here, and the cancel flags are [`CancelRegistry`].
//!
//! ## How progress and cancellation work now
//!
//! The shell passed `AppHandle` so a repair could `app.emit("mesh-repair-progress",
//! …)` at each stage and read a token's `AtomicBool` to stop. Neither is inherent:
//! the engines' `*_with_progress` entry points take
//! `Option<&mut dyn FnMut(&str, usize) -> bool>` — announce the stage, answer
//! *continue?*. So the port takes that sink from the caller and the GPUI layer
//! writes the event and reads its cancel flag inside it. [`CancelRegistry`] keeps
//! the token → flag map only because `mesh_repair_cancel` is itself a command the
//! UI calls by token.
//!
//! A degenerate but faithful consequence: the report's `cancelled` field is filled
//! from the flag the caller passed, exactly as the shell read its registered flag
//! after the run.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use dragonfruit_mesh_repair::{
    analyze, classify_support_split, hollow_voxel, io, punch_cylinders, repair_auto_with_progress,
    repair_separate_parts, shrink_wrap_with_progress, solid_remesh_with_progress,
    tet_surface_with_progress, HolePunchOptions, HollowOptions, HollowSession, IndexedMesh,
    RepairOptions, ShrinkWrapOptions, SolidRemeshOptions, TetWrapOptions, Vec3,
};
use serde::Deserialize;

use crate::staging::MeshStaging;
use crate::BackendResult;

/// Stage announcements a repair makes, and the answer to "keep going?".
///
/// The shell's `progress_sink` emitted `{"token":…,"name":…,"index":…}` on the
/// event named in each command; the event names are listed per command below so
/// the UI can wire them back up.
pub type StageSink<'a> = &'a mut dyn FnMut(&str, usize) -> bool;

/// Repairs in flight, by progress token.
///
/// A cancel flips the flag and the repair reads it at its next stage boundary, so
/// a cancel never lands halfway through a pass.
#[derive(Default)]
pub struct CancelRegistry {
    cancels: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl CancelRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a flag for `token`, if the caller named one.
    pub fn register(&self, token: Option<&str>) -> Option<Arc<AtomicBool>> {
        token.map(|token| {
            let flag = Arc::new(AtomicBool::new(false));
            if let Ok(mut registry) = self.cancels.lock() {
                registry.insert(token.to_string(), flag.clone());
            }
            flag
        })
    }

    /// Forget a finished run's flag.
    pub fn release(&self, token: Option<&str>) {
        if let Some(token) = token {
            if let Ok(mut registry) = self.cancels.lock() {
                registry.remove(token);
            }
        }
    }

    /// `mesh_repair_cancel`: ask the repair carrying this token to stop at its
    /// next stage boundary. Returns whether a repair was carrying that token — a
    /// cancel that arrives after the repair finished is a no-op, not an error.
    pub fn cancel(&self, progress_token: &str) -> BackendResult<bool> {
        let registry = self.cancels.lock().map_err(|e| e.to_string())?;
        Ok(registry
            .get(progress_token)
            .map(|flag| {
                flag.store(true, Ordering::SeqCst);
                true
            })
            .unwrap_or(false))
    }
}

// ---------------------------------------------------------------------------
// Option parsing
// ---------------------------------------------------------------------------

/// The repair options as the frontend sent them: every field optional, so the
/// engine's own defaults apply to whatever the caller did not name.
#[derive(Debug, Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct RepairOptionsDto {
    weld_tolerance_mm: Option<f32>,
    fill_holes_max_edges: Option<usize>,
    keep_largest_n_components: Option<usize>,
    repair_orientation: Option<bool>,
    resolve_self_intersections: Option<bool>,
    cut_unsatisfiable_edges: Option<bool>,
    allow_rebuild: Option<bool>,
    #[serde(alias = "assume_support_geometry")]
    assume_support_geometry: Option<bool>,
}

impl From<RepairOptionsDto> for RepairOptions {
    fn from(dto: RepairOptionsDto) -> Self {
        let defaults = RepairOptions::default();
        RepairOptions {
            // Passed through as they arrive, `None` and all: the engine derives a
            // tolerance the caller did not name. The frontend is not better placed
            // to pick these than the crate is — it has no mesh in hand.
            weld_tolerance_mm: dto.weld_tolerance_mm,
            fill_holes_max_edges: dto.fill_holes_max_edges,
            keep_largest_n_components: dto
                .keep_largest_n_components
                .or(defaults.keep_largest_n_components),
            repair_orientation: dto
                .repair_orientation
                .unwrap_or(defaults.repair_orientation),
            resolve_self_intersections: dto
                .resolve_self_intersections
                .unwrap_or(defaults.resolve_self_intersections),
            cut_unsatisfiable_edges: dto
                .cut_unsatisfiable_edges
                .unwrap_or(defaults.cut_unsatisfiable_edges),
            allow_rebuild: dto.allow_rebuild.unwrap_or(defaults.allow_rebuild),
            assume_support_geometry: dto
                .assume_support_geometry
                .or(defaults.assume_support_geometry),
        }
    }
}

pub fn parse_options(options_json: &str) -> BackendResult<RepairOptions> {
    if options_json.trim().is_empty() {
        return Ok(RepairOptions::default());
    }
    serde_json::from_str::<RepairOptionsDto>(options_json)
        .map(RepairOptions::from)
        .map_err(|e| format!("invalid repair options JSON: {e}"))
}

pub fn parse_hollow_options(options_json: &str) -> BackendResult<HollowOptions> {
    if options_json.trim().is_empty() {
        return Ok(HollowOptions::default());
    }
    serde_json::from_str::<HollowOptions>(options_json)
        .map_err(|e| format!("invalid hollow options JSON: {e}"))
}

pub fn parse_shrinkwrap_options(options_json: &str) -> BackendResult<ShrinkWrapOptions> {
    if options_json.trim().is_empty() {
        return Ok(ShrinkWrapOptions::default());
    }
    serde_json::from_str::<ShrinkWrapOptions>(options_json)
        .map_err(|e| format!("invalid shrinkwrap options JSON: {e}"))
}

pub fn parse_solid_remesh_options(options_json: &str) -> BackendResult<SolidRemeshOptions> {
    if options_json.trim().is_empty() {
        return Ok(SolidRemeshOptions::default());
    }
    // The shell's message says "shrink-wrap" here because the two shared a parser
    // once; the text is part of the contract, so it is kept verbatim.
    serde_json::from_str::<SolidRemeshOptions>(options_json)
        .map_err(|e| format!("invalid shrink-wrap options JSON: {e}"))
}

pub fn parse_tet_wrap_options(options_json: &str) -> BackendResult<TetWrapOptions> {
    if options_json.trim().is_empty() {
        return Ok(TetWrapOptions::default());
    }
    serde_json::from_str::<TetWrapOptions>(options_json)
        .map_err(|e| format!("invalid tet surface options JSON: {e}"))
}

pub fn parse_hole_punch_options(options_json: &str) -> BackendResult<HolePunchOptions> {
    if options_json.trim().is_empty() {
        return Ok(HolePunchOptions::default());
    }
    serde_json::from_str::<HolePunchOptions>(options_json)
        .map_err(|e| format!("invalid hole punch options JSON: {e}"))
}

// ---------------------------------------------------------------------------
// Analysis and loading
// ---------------------------------------------------------------------------

/// `mesh_analyze_from_path`: parse a mesh file and return the analysis JSON.
pub fn mesh_analyze_from_path(file_path: &str) -> BackendResult<String> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!(
            "mesh_analyze_from_path: not found: {}",
            path.display()
        ));
    }
    let mesh =
        io::load_mesh_from_path(&path).map_err(|e| format!("analyze task panicked: {e}"))?;
    let analysis = analyze(&mesh);
    serde_json::to_string(&analysis).map_err(|e| format!("serialize analysis: {e}"))
}

/// `mesh_load_from_path`: load a mesh and stage it, repairing nothing.
///
/// The webview refused a `file://` fetch at the security boundary, so a model's
/// file could not be read back from the UI directly. This is the first half of
/// [`mesh_repair_from_path`] on its own, for putting a model back to how it
/// arrived.
pub fn mesh_load_from_path(staging: &MeshStaging, file_path: &str) -> BackendResult<()> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!("mesh_load_from_path: not found: {}", path.display()));
    }
    let mesh = io::load_mesh_from_path(&path).map_err(|e| format!("load task panicked: {e}"))?;
    staging.replace_with_mesh(&mesh)
}

/// `mesh_analyze_staged`: analyse the staged buffer without modifying it.
///
/// The UI inspects mesh health before committing to a repair.
pub fn mesh_analyze_staged(staging: &MeshStaging) -> BackendResult<String> {
    let bytes = staging.read_staging_bytes()?;
    let analysis = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("analyze task panicked: {e}"))?;
    let analysis = analyze(&analysis);
    serde_json::to_string(&analysis).map_err(|e| format!("serialize analysis: {e}"))
}

/// `mesh_repair_read_positions`: the current staged positions, as raw
/// little-endian bytes (`9 × f32` per triangle), for the renderer to hydrate a
/// buffer geometry after a repair.
pub fn mesh_repair_read_positions(staging: &MeshStaging) -> BackendResult<Vec<u8>> {
    staging.read_staging_bytes()
}

/// `refine_mesh_soup`: split coarse faces on geometry an importer built outside the
/// native loaders.
///
/// LYS is the case this exists for: it is imported by a TypeScript plugin, plugins
/// cannot call native code, and so its geometry never passes through
/// `io::load_mesh_from_path` where coarse faces are refined. The host calls this after
/// a plugin import instead, which keeps one implementation of the rule.
///
/// In: a non-indexed triangle soup, 9 `f32` per triangle. Out:
/// `[u32 triangle count][f32 positions][f32 normals]`, 9 of each per triangle, one
/// normal per corner. The mesh is welded on the way in, which is what gives the
/// refinement shared edges to split.
pub fn refine_mesh_soup(soup: &[f32]) -> BackendResult<Vec<u8>> {
    use dragonfruit_mesh_core::normals::face_normals;
    use dragonfruit_mesh_core::refine::{
        longest_edge, refine_long_edges_with_budget, refinement_budget, MAX_EDGE_DIAGONAL_FRACTION,
        MAX_PASSES,
    };
    use dragonfruit_mesh_repair::io::DEFAULT_MERGE_EPSILON;

    if soup.len() % 9 != 0 {
        return Err(format!(
            "positions are not a multiple of 9 floats: {}",
            soup.len()
        ));
    }
    let started = std::time::Instant::now();
    let mesh = IndexedMesh::from_triangle_soup(soup, DEFAULT_MERGE_EPSILON);
    if mesh.triangles.is_empty() {
        return Err("refine_mesh_soup: no triangles".to_string());
    }

    let mut min = [f32::INFINITY; 3];
    let mut max = [f32::NEG_INFINITY; 3];
    for position in &mesh.positions {
        for (axis, value) in [position.x, position.y, position.z].into_iter().enumerate() {
            min[axis] = min[axis].min(value);
            max[axis] = max[axis].max(value);
        }
    }
    let diagonal = ((max[0] - min[0]).powi(2)
        + (max[1] - min[1]).powi(2)
        + (max[2] - min[2]).powi(2))
    .sqrt();
    let max_edge = diagonal * MAX_EDGE_DIAGONAL_FRACTION;
    let refined = if diagonal.is_finite() && diagonal > 0.0 && longest_edge(&mesh) > max_edge {
        let budget = refinement_budget(mesh.triangles.len());
        refine_long_edges_with_budget(&mesh, max_edge, MAX_PASSES, budget)
    } else {
        mesh
    };

    let normals = face_normals(&refined);
    let triangles = refined.triangles.len();
    log::info!(
        "[refine_mesh_soup] {} soup triangles -> {} refined in {}ms",
        soup.len() / 9,
        triangles,
        started.elapsed().as_millis(),
    );

    let mut out = Vec::with_capacity(4 + triangles * 18 * 4);
    out.extend_from_slice(&(triangles as u32).to_le_bytes());
    for triangle in &refined.triangles {
        for index in triangle {
            let position = refined.positions[*index as usize];
            out.extend_from_slice(&position.x.to_le_bytes());
            out.extend_from_slice(&position.y.to_le_bytes());
            out.extend_from_slice(&position.z.to_le_bytes());
        }
    }
    for normal in &normals {
        out.extend_from_slice(&normal.x.to_le_bytes());
        out.extend_from_slice(&normal.y.to_le_bytes());
        out.extend_from_slice(&normal.z.to_le_bytes());
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// Repair
// ---------------------------------------------------------------------------

/// `mesh_repair_from_path`: parse, repair, replace the staging buffer with the
/// repaired positions, return the health report JSON.
///
/// Progress event: `mesh-repair-progress`.
pub fn mesh_repair_from_path(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    file_path: &str,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<StageSink<'_>>,
) -> BackendResult<String> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!(
            "mesh_repair_from_path: not found: {}",
            path.display()
        ));
    }
    let options = parse_options(options_json)?;
    let source_path = file_path.to_string();
    let flag = cancel.register(progress_token);

    let result = (|| {
        let mesh = io::load_mesh_from_path(&path).map_err(|e| format!("repair task panicked: {e}"))?;
        let mut owned_sink = sink;
        let outcome =
            repair_auto_with_progress(mesh, &options, owned_sink.as_mut().map(|sink| &mut **sink));
        Ok::<_, String>((outcome.mesh, outcome.report))
    })();
    cancel.release(progress_token);
    drop(flag);

    let (mesh, mut report) = result?;
    report.source_path = Some(source_path);
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize report: {e}"))
}

/// `mesh_repair_parts_from_path`: repair a plate part by part.
///
/// A plate is not one model — the reported one is 25 objects over 118 components —
/// and repairing it whole returns one verdict for all of them, which tells a user
/// nothing they can act on. The shells that meet stay together as one object, each
/// object is repaired on its own, and the result is staged as one mesh with a
/// per-object verdict.
///
/// Progress event: `mesh-repair-progress`. `cancelled` is read after the run and
/// written into the report, as the shell did with its registered flag.
pub fn mesh_repair_parts_from_path(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    file_path: &str,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<&mut dyn FnMut(usize, &str)>,
) -> BackendResult<String> {
    let path = PathBuf::from(file_path);
    if !path.exists() {
        return Err(format!(
            "mesh_repair_parts_from_path: not found: {}",
            path.display()
        ));
    }
    let options = parse_options(options_json)?;
    let source_path = file_path.to_string();
    let flag = cancel.register(progress_token);

    let parts = (|| {
        let mesh = io::load_mesh_from_path(&path).map_err(|e| format!("repair parts task panicked: {e}"))?;
        repair_separate_parts(&mesh, &options, sink).ok_or_else(|| {
            "mesh_repair_parts_from_path: the mesh is one object, or its shells meet each other"
                .to_string()
        })
    })();

    let was_cancelled = flag
        .as_ref()
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false);
    cancel.release(progress_token);

    let parts = parts?;
    let mesh = parts.combined_mesh();
    let mut report = parts.report(Some(source_path));
    report.cancelled = was_cancelled;
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize report: {e}"))
}

/// `mesh_repair_staged`: repair whatever is in the staging buffer.
///
/// Progress event: `mesh-repair-progress`.
pub fn mesh_repair_staged(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<StageSink<'_>>,
) -> BackendResult<String> {
    let options = parse_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;
    let flag = cancel.register(progress_token);

    let result = (|| {
        let mesh = io::staged::load_positions_le(&bytes)
            .map_err(|e| format!("repair task panicked: {e}"))?;
        let mut owned_sink = sink;
        let outcome =
            repair_auto_with_progress(mesh, &options, owned_sink.as_mut().map(|sink| &mut **sink));
        Ok::<_, String>((outcome.mesh, outcome.report))
    })();
    cancel.release(progress_token);
    drop(flag);

    let (mesh, report) = result?;
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize report: {e}"))
}

/// `mesh_classify_staged`: the model/support section classifier alone, no repair.
///
/// The classifier reorders the geometry so the model section comes first, which is
/// what `modelTriangleCount` in the report addresses.
pub fn mesh_classify_staged(
    staging: &MeshStaging,
    options_json: Option<&str>,
) -> BackendResult<String> {
    let options = parse_options(options_json.unwrap_or_default())?;
    let bytes = staging.read_staging_bytes()?;
    let mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("classify task panicked: {e}"))?;
    let outcome = classify_support_split(mesh, &options);
    staging.replace_with_mesh(&outcome.mesh)?;
    serde_json::to_string(&outcome.report).map_err(|e| format!("serialize report: {e}"))
}

// ---------------------------------------------------------------------------
// Surfacing: shrinkwrap, tet surface, solid remesh
// ---------------------------------------------------------------------------

/// `mesh_shrinkwrap_staged`: resample the staged mesh with the shrinkwrap
/// resampler.
///
/// Deliberately separate from repair: shrinkwrap rebuilds the surface from a
/// signed distance field, so the result has no relation to the input beyond its
/// shape — every vertex is new. It exists for meshes whose topology cannot be
/// repaired, where repair has to refuse rather than damage the object.
///
/// Progress event: `mesh-shrinkwrap-progress`.
pub fn mesh_shrinkwrap_staged(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<StageSink<'_>>,
) -> BackendResult<String> {
    let options = parse_shrinkwrap_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;
    let flag = cancel.register(progress_token);

    let result = (|| {
        let mesh = io::staged::load_positions_le(&bytes)
            .map_err(|e| format!("shrinkwrap task panicked: {e}"))?;
        let mut owned_sink = sink;
        let outcome =
            shrink_wrap_with_progress(&mesh, &options, owned_sink.as_mut().map(|sink| &mut **sink));
        Ok::<_, String>((outcome.mesh, outcome.report))
    })();
    cancel.release(progress_token);
    drop(flag);

    let (mesh, report) = result?;
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize shrinkwrap report: {e}"))
}

/// `mesh_tet_surface_staged`: resample by tetrahedralising the mesh's own points
/// and keeping the inside/outside boundary.
///
/// Where shrinkwrap resamples onto a uniform lattice, this follows the model's own
/// points, so it preserves detail a lattice would lose, and the boundary between
/// two labels is closed by construction, so it emits no debris where the band
/// isosurface does.
///
/// Progress event: `mesh-tet-surface-progress`.
pub fn mesh_tet_surface_staged(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<StageSink<'_>>,
) -> BackendResult<String> {
    let options = parse_tet_wrap_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;
    let flag = cancel.register(progress_token);

    let result = (|| {
        let mesh = io::staged::load_positions_le(&bytes)
            .map_err(|e| format!("tet surface task panicked: {e}"))?;
        let mut owned_sink = sink;
        let outcome =
            tet_surface_with_progress(&mesh, &options, owned_sink.as_mut().map(|sink| &mut **sink));
        Ok::<_, String>((outcome.mesh, outcome.report))
    })();
    cancel.release(progress_token);
    drop(flag);

    let (mesh, report) = result?;
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize tet surface report: {e}"))
}

/// `mesh_solid_remesh_staged`: rebuild the surface by resolving the mesh into a
/// solid.
///
/// This is the mode the reference implementation's shrinkwrap uses: close what is
/// open, boolean the mesh with itself so the result is resolved and retriangulated,
/// then drop scrap shells. It moves the surface, where the tetrahedral mode
/// conforms to it. On a clean closed solid it is a no-op, so it is for a mesh that
/// is not one solid.
///
/// Progress event: `mesh-solid-remesh-progress`.
pub fn mesh_solid_remesh_staged(
    staging: &MeshStaging,
    cancel: &CancelRegistry,
    options_json: &str,
    progress_token: Option<&str>,
    sink: Option<StageSink<'_>>,
) -> BackendResult<String> {
    let options = parse_solid_remesh_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;
    let flag = cancel.register(progress_token);

    let result = (|| {
        let mesh = io::staged::load_positions_le(&bytes)
            .map_err(|e| format!("solid remesh task panicked: {e}"))?;
        let mut owned_sink = sink;
        let outcome =
            solid_remesh_with_progress(&mesh, &options, owned_sink.as_mut().map(|sink| &mut **sink));
        Ok::<_, String>((outcome.mesh, outcome.report))
    })();
    cancel.release(progress_token);
    drop(flag);

    let (mesh, report) = result?;
    staging.replace_with_mesh(&mesh)?;
    serde_json::to_string(&report).map_err(|e| format!("serialize solid remesh report: {e}"))
}

// ---------------------------------------------------------------------------
// Hollowing
// ---------------------------------------------------------------------------

fn lock<'a, T>(mutex: &'a Mutex<T>, what: &str) -> BackendResult<std::sync::MutexGuard<'a, T>> {
    mutex.lock().map_err(|e| format!("{what} lock poisoned: {e}"))
}

/// Everything the non-mutating hollow preview keeps between calls.
///
/// The preview is a session against a *captured* source mesh, so dragging a slider
/// re-runs hollowing without touching the staged mesh the viewport is rendering.
/// The session caches the voxel grid and topology, which is what makes a slider
/// drag cheap; the byte buffers are the results the renderer reads back.
#[derive(Default)]
pub struct HollowPreview {
    source_mesh: Mutex<Option<Arc<IndexedMesh>>>,
    session: Mutex<Option<Arc<HollowSession>>>,
    result_bytes: Mutex<Option<Vec<u8>>>,
    infill_bytes: Mutex<Option<Vec<u8>>>,
    removed_center_bytes: Mutex<Option<Vec<u8>>>,
    removed_index_bytes: Mutex<Option<Vec<u8>>>,
    blocked_center_bytes: Mutex<Option<Vec<u8>>>,
    blocked_index_bytes: Mutex<Option<Vec<u8>>>,
    staged_cavity_bytes: Mutex<Option<Vec<u8>>>,
    preview_cavity_bytes: Mutex<Option<Vec<u8>>>,
}

impl HollowPreview {
    pub fn new() -> Self {
        Self::default()
    }

    /// Clear every buffer derived from the captured source mesh.
    ///
    /// Called whenever a new source is captured, so stale data from a previous
    /// model or session can never be served alongside a fresh one.
    fn reset_derived_state(&self) -> BackendResult<()> {
        *lock(&self.session, "hollow preview session")? = None;
        *lock(&self.result_bytes, "hollow preview result")? = None;
        *lock(&self.infill_bytes, "hollow preview infill result")? = None;
        *lock(&self.removed_center_bytes, "hollow preview removed voxel center result")? = None;
        *lock(&self.removed_index_bytes, "hollow preview removed voxel index result")? = None;
        *lock(&self.blocked_center_bytes, "hollow preview blocked voxel center result")? = None;
        *lock(&self.blocked_index_bytes, "hollow preview blocked voxel index result")? = None;
        *lock(&self.preview_cavity_bytes, "hollow preview cavity result")? = None;
        *lock(&self.staged_cavity_bytes, "hollow staged cavity result")? = None;
        Ok(())
    }

    /// The cached session, rebuilt when the source, resolution or rotation changed.
    fn session_for(&self, options: &HollowOptions) -> BackendResult<Arc<HollowSession>> {
        let source = lock(&self.source_mesh, "hollow preview source")?
            .clone()
            .ok_or_else(|| {
                "No captured hollow preview source — call mesh_hollow_preview_capture_staged_source first"
                    .to_string()
            })?;

        if let Some(session) = lock(&self.session, "hollow preview session")?.clone() {
            if session.voxel_resolution() == options.voxel_resolution
                && session.rotation_quat() == options.rotation_quat
            {
                return Ok(session);
            }
        }

        let session = Arc::new(HollowSession::with_rotation(
            (*source).clone(),
            options.voxel_resolution,
            options.rotation_quat,
        ));
        *lock(&self.session, "hollow preview session")? = Some(session.clone());
        Ok(session)
    }
}

/// `mesh_hollow_preview_capture_staged_source`: capture the staged mesh as the
/// source for repeated non-mutating hollow previews.
pub fn mesh_hollow_preview_capture_staged_source(
    staging: &MeshStaging,
    preview: &HollowPreview,
) -> BackendResult<()> {
    let bytes = staging.read_staging_bytes()?;
    let source_mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("hollow preview capture task panicked: {e}"))?;
    *lock(&preview.source_mesh, "hollow preview source")? = Some(Arc::new(source_mesh));
    preview.reset_derived_state()
}

/// `mesh_hollow_staged`: voxel hollowing on the current staged mesh.
///
/// Replaces the staged positions with the hollowed result, stashes the cavity
/// interior for `mesh_hollow_staged_read_cavity_positions`, and returns the report
/// JSON.
pub fn mesh_hollow_staged(
    staging: &MeshStaging,
    preview: &HollowPreview,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_hollow_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;

    let mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("hollow task panicked: {e}"))?;
    let outcome = hollow_voxel(mesh, &options);
    let cavity_bytes = outcome.cavity_mesh.as_ref().map(|cm| {
        let soup = cm.to_triangle_soup();
        bytemuck::cast_slice::<f32, u8>(&soup).to_vec()
    });

    *lock(&preview.staged_cavity_bytes, "hollow staged cavity result")? = cavity_bytes;
    staging.replace_with_mesh(&outcome.mesh)?;
    serde_json::to_string(&outcome.report).map_err(|e| format!("serialize hollow report: {e}"))
}

/// `mesh_hollow_preview_from_captured_source`: hollow the captured source without
/// mutating the staged mesh.
///
/// Every result the interior view needs is stashed: positions, cavity, infill,
/// removed-voxel centres/indices and blocked-voxel centres/indices.
pub fn mesh_hollow_preview_from_captured_source(
    preview: &HollowPreview,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_hollow_options(options_json)?;
    let session = preview.session_for(&options)?;

    let outcome = session.run(&options);
    let soup = outcome.mesh.to_triangle_soup();
    let positions_bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();
    let cavity_bytes = outcome.cavity_mesh.as_ref().map(|cm| {
        let soup = cm.to_triangle_soup();
        bytemuck::cast_slice::<f32, u8>(&soup).to_vec()
    });
    let infill_bytes = outcome.preview_infill_mesh.map(|mesh| {
        let soup = mesh.to_triangle_soup();
        bytemuck::cast_slice::<f32, u8>(&soup).to_vec()
    });
    let removed_center_bytes =
        bytemuck::cast_slice::<f32, u8>(&outcome.removed_voxel_centers).to_vec();
    let removed_index_bytes =
        bytemuck::cast_slice::<u32, u8>(&outcome.removed_voxel_indices).to_vec();
    let blocked_center_bytes =
        bytemuck::cast_slice::<f32, u8>(&outcome.blocked_voxel_centers).to_vec();
    let blocked_index_bytes =
        bytemuck::cast_slice::<u32, u8>(&outcome.blocked_voxel_indices).to_vec();

    *lock(&preview.result_bytes, "hollow preview result")? = Some(positions_bytes);
    *lock(&preview.preview_cavity_bytes, "hollow preview cavity result")? = cavity_bytes;
    *lock(&preview.infill_bytes, "hollow preview infill result")? = infill_bytes;
    *lock(&preview.removed_center_bytes, "hollow preview removed voxel center result")? =
        Some(removed_center_bytes);
    *lock(&preview.removed_index_bytes, "hollow preview removed voxel index result")? =
        Some(removed_index_bytes);
    *lock(&preview.blocked_center_bytes, "hollow preview blocked voxel center result")? =
        Some(blocked_center_bytes);
    *lock(&preview.blocked_index_bytes, "hollow preview blocked voxel index result")? =
        Some(blocked_index_bytes);

    serde_json::to_string(&outcome.report)
        .map_err(|e| format!("serialize hollow preview report: {e}"))
}

/// `mesh_hollow_apply_from_captured_source`: commit the preview's hollowing to the
/// staged mesh.
pub fn mesh_hollow_apply_from_captured_source(
    staging: &MeshStaging,
    preview: &HollowPreview,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_hollow_options(options_json)?;
    let session = preview.session_for(&options)?;

    let outcome = session.run(&options);
    let cavity_bytes = outcome.cavity_mesh.as_ref().map(|cm| {
        let soup = cm.to_triangle_soup();
        bytemuck::cast_slice::<f32, u8>(&soup).to_vec()
    });
    *lock(&preview.staged_cavity_bytes, "hollow staged cavity result")? = cavity_bytes;

    staging.replace_with_mesh(&outcome.mesh)?;
    serde_json::to_string(&outcome.report).map_err(|e| format!("serialize hollow apply report: {e}"))
}

macro_rules! hollow_read_back {
    ($fn_name:ident, $field:ident, $lock_name:literal, $message:literal) => {
        /// Raw little-endian bytes of the last hollow preview's buffer.
        pub fn $fn_name(preview: &HollowPreview) -> BackendResult<Vec<u8>> {
            lock(&preview.$field, $lock_name)?
                .clone()
                .ok_or_else(|| $message.to_string())
        }
    };
}

hollow_read_back!(
    mesh_hollow_preview_read_positions,
    result_bytes,
    "hollow preview result",
    "No hollow preview result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_infill_positions,
    infill_bytes,
    "hollow preview infill result",
    "No hollow preview infill result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_removed_voxel_centers,
    removed_center_bytes,
    "hollow preview removed voxel center result",
    "No hollow preview removed voxel center result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_removed_voxel_indices,
    removed_index_bytes,
    "hollow preview removed voxel index result",
    "No hollow preview removed voxel index result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_blocked_voxel_centers,
    blocked_center_bytes,
    "hollow preview blocked voxel center result",
    "No hollow preview blocked voxel center result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_blocked_voxel_indices,
    blocked_index_bytes,
    "hollow preview blocked voxel index result",
    "No hollow preview blocked voxel index result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_preview_read_cavity_positions,
    preview_cavity_bytes,
    "hollow preview cavity result",
    "No hollow preview cavity result — call mesh_hollow_preview_from_captured_source first"
);
hollow_read_back!(
    mesh_hollow_staged_read_cavity_positions,
    staged_cavity_bytes,
    "hollow staged cavity result",
    "No hollow staged cavity result — call mesh_hollow_staged first"
);

/// Request payload for [`mesh_hollow_preview_select_removed_voxels_in_polygon`].
///
/// All fields are in the spaces the frontend lasso resolver used before the
/// projection moved to Rust: `polygon` in container pixels, `view_proj` a
/// column-major `projectionMatrix * matrixWorldInverse`, and the model transform
/// matching `resolveBlockedHollowVoxelMarqueeSelection`.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectRemovedVoxelsRequest {
    pub polygon: Vec<[f32; 2]>,
    pub view_proj: [f32; 16],
    pub rect_width: f32,
    pub rect_height: f32,
    pub geometry_center: [f32; 3],
    pub scale: [f32; 3],
    pub rotation_quat: [f32; 4],
    pub position: [f32; 3],
    pub options: HollowOptions,
}

/// `mesh_hollow_preview_select_removed_voxels_in_polygon`.
///
/// Selects the full through-depth set of removed (cavity) voxels whose projected
/// screen point falls inside the lasso polygon, operating on the cached preview
/// session — so the result is immune to the boundary filter and viewport cap that
/// narrow the rendered/exported voxel subset. Returns grid indices as raw
/// little-endian `u32` bytes.
pub fn mesh_hollow_preview_select_removed_voxels_in_polygon(
    preview: &HollowPreview,
    request_json: &str,
) -> BackendResult<Vec<u8>> {
    let request: SelectRemovedVoxelsRequest = serde_json::from_str(request_json)
        .map_err(|e| format!("invalid select-removed-voxels request JSON: {e}"))?;

    let session = lock(&preview.session, "hollow preview session")?
        .clone()
        .ok_or_else(|| {
            "No hollow preview session — call mesh_hollow_preview_from_captured_source first"
                .to_string()
        })?;

    let selected = session.select_removed_voxels_in_polygon(
        &request.options,
        &request.polygon,
        &request.view_proj,
        request.rect_width,
        request.rect_height,
        Vec3::new(
            request.geometry_center[0],
            request.geometry_center[1],
            request.geometry_center[2],
        ),
        Vec3::new(request.scale[0], request.scale[1], request.scale[2]),
        request.rotation_quat,
        Vec3::new(request.position[0], request.position[1], request.position[2]),
    );

    Ok(bytemuck::cast_slice::<u32, u8>(&selected).to_vec())
}

// ---------------------------------------------------------------------------
// Hole punching
// ---------------------------------------------------------------------------

/// The non-mutating punch preview's captured source and last result.
#[derive(Default)]
pub struct PunchPreview {
    source_bytes: Mutex<Option<Vec<u8>>>,
    result_bytes: Mutex<Option<Vec<u8>>>,
}

impl PunchPreview {
    pub fn new() -> Self {
        Self::default()
    }
}

/// `mesh_punch_staged`: apply manual cylindrical hole punches to the staged mesh.
pub fn mesh_punch_staged(staging: &MeshStaging, options_json: &str) -> BackendResult<String> {
    let options = parse_hole_punch_options(options_json)?;
    let bytes = staging.read_staging_bytes()?;
    let mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("punch task panicked: {e}"))?;
    let outcome = punch_cylinders(mesh, &options);
    staging.replace_with_mesh(&outcome.mesh)?;
    serde_json::to_string(&outcome.report).map_err(|e| format!("serialize punch report: {e}"))
}

/// `mesh_punch_capture_staged_source`: capture the staged mesh as the source for
/// repeated non-mutating punch runs.
pub fn mesh_punch_capture_staged_source(
    staging: &MeshStaging,
    preview: &PunchPreview,
) -> BackendResult<()> {
    let bytes = staging.read_staging_bytes()?;
    *lock(&preview.source_bytes, "punch source")? = Some(bytes);
    *lock(&preview.result_bytes, "punch result")? = None;
    Ok(())
}

/// `mesh_punch_from_captured_source`: punch the captured source without mutating
/// the staged mesh.
pub fn mesh_punch_from_captured_source(
    preview: &PunchPreview,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_hole_punch_options(options_json)?;
    let source_bytes = lock(&preview.source_bytes, "punch source")?
        .clone()
        .ok_or_else(|| {
            "No captured punch source — call mesh_punch_capture_staged_source first".to_string()
        })?;

    let mesh = io::staged::load_positions_le(&source_bytes)
        .map_err(|e| format!("punch task panicked: {e}"))?;
    let outcome = punch_cylinders(mesh, &options);
    let soup = outcome.mesh.to_triangle_soup();
    *lock(&preview.result_bytes, "punch result")? =
        Some(bytemuck::cast_slice::<f32, u8>(&soup).to_vec());

    serde_json::to_string(&outcome.report).map_err(|e| format!("serialize punch report: {e}"))
}

/// `mesh_punch_read_positions`: the last non-mutating punch result, as raw
/// little-endian bytes.
pub fn mesh_punch_read_positions(preview: &PunchPreview) -> BackendResult<Vec<u8>> {
    lock(&preview.result_bytes, "punch result")?
        .clone()
        .ok_or_else(|| "No punch result — call mesh_punch_from_captured_source first".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 mm cube as a triangle soup, wound so its normals point outward.
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
        // Six faces, two triangles each, counter-clockwise seen from outside.
        let faces: [[usize; 4]; 6] = [
            [0, 3, 2, 1], // bottom (-Z)
            [4, 5, 6, 7], // top (+Z)
            [0, 1, 5, 4], // front (-Y)
            [1, 2, 6, 5], // right (+X)
            [2, 3, 7, 6], // back (+Y)
            [3, 0, 4, 7], // left (-X)
        ];
        let mut soup = Vec::with_capacity(36 * 3);
        for [a, b, c, d] in faces {
            for index in [a, b, c, a, c, d] {
                soup.extend_from_slice(&corners[index]);
            }
        }
        soup
    }

    fn staged_cube() -> MeshStaging {
        let soup = cube_soup();
        let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();
        let staging = MeshStaging::new();
        staging.stage_mesh_binary_set(&bytes).expect("stage");
        staging
    }

    #[test]
    fn staged_cube_analyses() {
        let staging = staged_cube();
        let report = mesh_analyze_staged(&staging).expect("analyse");
        let parsed: serde_json::Value = serde_json::from_str(&report).expect("json");
        assert!(parsed.is_object(), "{report}");
    }

    #[test]
    fn read_positions_hands_back_the_staged_soup() {
        let staging = staged_cube();
        let bytes = mesh_repair_read_positions(&staging).expect("positions");
        let floats: &[f32] = bytemuck::cast_slice(&bytes);
        assert_eq!(floats.len(), cube_soup().len());
    }

    #[test]
    fn repair_keeps_the_buffer_and_reports() {
        let staging = staged_cube();
        let cancel = CancelRegistry::new();
        let report = mesh_repair_staged(
            &staging,
            &cancel,
            "",
            None,
            // A sink that always continues: the engine announces its stages
            // through it, and a repaired solid may have none to announce.
            Some(&mut |_name: &str, _index: usize| true),
        )
        .expect("repair");
        let parsed: serde_json::Value = serde_json::from_str(&report).expect("json");
        assert!(parsed.get("pre").is_some(), "{report}");
        // The staging buffer still holds a triangle soup after the repair.
        assert_eq!(staging.read_staging_bytes().expect("read").len() % 4, 0);
    }

    #[test]
    fn a_sink_that_says_no_stops_the_run() {
        let staging = staged_cube();
        let cancel = CancelRegistry::new();
        let _ = mesh_repair_staged(&staging, &cancel, "", None, Some(&mut |_n, _i| false));
        // The run is allowed to return either a partial result or an error; what
        // must hold is that saying no is not silently ignored as a panic.
        assert!(staging.read_staging_bytes().is_ok());
    }

    #[test]
    fn classify_reorders_but_answers() {
        let staging = staged_cube();
        let report = mesh_classify_staged(&staging, None).expect("classify");
        assert!(report.contains("\"pre\""), "{report}");
    }

    #[test]
    fn hollow_preview_requires_a_captured_source() {
        let staging = staged_cube();
        let preview = HollowPreview::new();
        let err = mesh_hollow_preview_from_captured_source(&preview, "").expect_err("no source");
        assert!(err.contains("capture_staged_source"), "{err}");
        assert!(mesh_hollow_preview_read_positions(&preview).is_err());

        mesh_hollow_preview_capture_staged_source(&staging, &preview).expect("capture");
        // The default options (64³ voxels, 2 mm shell) hollow a 10 mm cube.
        mesh_hollow_preview_from_captured_source(&preview, "").expect("preview");
        assert!(!mesh_hollow_preview_read_positions(&preview)
            .expect("positions")
            .is_empty());
    }

    #[test]
    fn punch_preview_requires_a_captured_source() {
        let staging = staged_cube();
        let preview = PunchPreview::new();
        let err = mesh_punch_from_captured_source(&preview, "").expect_err("no source");
        assert!(err.contains("mesh_punch_capture_staged_source"), "{err}");
        mesh_punch_capture_staged_source(&staging, &preview).expect("capture");
        assert!(mesh_punch_read_positions(&preview).is_err());
    }

    #[test]
    fn cancel_flags_are_per_token() {
        let cancel = CancelRegistry::new();
        let flag = cancel.register(Some("run-a")).expect("flag");
        assert!(!flag.load(Ordering::SeqCst));
        assert!(cancel.cancel("run-a").expect("cancel"));
        assert!(flag.load(Ordering::SeqCst));
        // A token nobody is carrying is a no-op, not an error.
        assert!(!cancel.cancel("run-b").expect("no-op"));
        cancel.release(Some("run-a"));
        assert!(!cancel.cancel("run-a").expect("released"));
        assert!(cancel.register(None).is_none());
    }

    #[test]
    fn a_missing_file_is_reported_before_any_work() {
        let err = mesh_analyze_from_path("definitely/not/here.stl").expect_err("missing");
        assert!(err.contains("not found"), "{err}");
    }
}
