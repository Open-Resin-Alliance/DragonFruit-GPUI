//! Organic cut — the `mesh_organic_cut_*` family.
//!
//! Ported from `src-tauri/src/mesh_repair.rs` lines 1300-1830. The cut itself is
//! `dragonfruit-organic-cut`; what lived in the shell was the preview protocol:
//! capture a source mesh, run non-mutating cuts and geodesic/membrane/tenon
//! previews against it, and hand the renderer raw byte buffers plus small JSON
//! status objects it parses by hand.
//!
//! Those hand-built JSON strings are kept verbatim — the renderer reads
//! `triangleCount`, `tenonKind`, `tenonFrame.{anchor,axis,u,v,tip,depth,maxTiltRad,halfDiagMm}`
//! and the rest by name, so the shapes are the contract.

use std::sync::{Arc, Mutex};

use dragonfruit_mesh_repair::{io, IndexedMesh, Vec3};
use dragonfruit_organic_cut::{
    organic_cut, GeodesicSolver, OrganicCutOptions, OrganicCutOutcome, TenonShape, TenonTilt,
};
use serde::Deserialize;

use crate::staging::MeshStaging;
use crate::BackendResult;

/// The empty answer for a tenon preview: no tenon, no frame, no membrane.
const NO_TENON_PREVIEW_JSON: &str =
    "{\"triangleCount\":0,\"jointTriangleCount\":0,\"tenonTriangleCount\":0,\"tenonKind\":\"none\",\"tenonFits\":true,\"tenonDetail\":\"\",\"tenonFrame\":null}";

/// Everything the cut preview keeps between calls.
///
/// `source_bytes` is the captured mesh as a staged soup; `parts_bytes` is the last
/// cut's parts, one soup each (a multi-loop cut that frees several pieces has more
/// than two); the rest are the geodesic/membrane/tenon buffers the renderer reads
/// back. The solver is cached because building it — parse, topology, vertex graph —
/// is O(mesh) and dominates a single query, which is what makes dragging a waypoint
/// cheap: each query is then only Dijkstra + path straightening.
#[derive(Default)]
pub struct OrganicCutState {
    source_bytes: Mutex<Option<Vec<u8>>>,
    parts_bytes: Mutex<Vec<Vec<u8>>>,
    geodesic_bytes: Mutex<Option<Vec<u8>>>,
    geodesic_solver: Mutex<Option<Arc<GeodesicSolver>>>,
    membrane_bytes: Mutex<Option<Vec<u8>>>,
    tenon_bytes: Mutex<Option<Vec<u8>>>,
}

impl OrganicCutState {
    pub fn new() -> Self {
        Self::default()
    }

    fn lock<'a, T>(
        mutex: &'a Mutex<T>,
        what: &str,
    ) -> BackendResult<std::sync::MutexGuard<'a, T>> {
        mutex.lock().map_err(|e| format!("{what} lock poisoned: {e}"))
    }
}

#[derive(Deserialize)]
struct GeodesicWaypointDto {
    position: [f32; 3],
}

fn default_smoothing_half() -> f32 {
    0.5
}

/// The waypoint/preview request the renderer sends for every cut preview.
///
/// Every optional field defaults exactly as the shell's did, including
/// `jointClearanceMm` accepting the old `thicknessMm` name — the cut was a wafer
/// whose thickness was structural, and saved sessions still send the old key.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeodesicRequestDto {
    #[serde(default)]
    points: Vec<GeodesicWaypointDto>,
    #[serde(default)]
    close: bool,
    /// Seam smoothing 0..1 (line corner-rounding).
    #[serde(default = "default_smoothing_half")]
    smoothing: f32,
    /// Membrane smoothing 0..1 (cutter-surface relaxation).
    #[serde(default = "default_smoothing_half")]
    membrane_smoothing: f32,
    /// Cut resolution multiplier (1..4).
    #[serde(default = "default_density_one")]
    density: f32,
    /// Extra clearance in mm for the mortise-and-tenon joint, on top of the
    /// tenon's own tolerance. Zero means the halves meet exactly. Previewed with
    /// the same number the cut spends, or the preview lies about the fit.
    #[serde(default, alias = "thicknessMm")]
    joint_clearance_mm: f32,
    /// When true, the preview also builds the registration tenon the cut would
    /// place, so the user sees it before cutting.
    #[serde(default)]
    generate_tenon: bool,
    #[serde(default = "default_tenon_width")]
    tenon_width_mm: f32,
    #[serde(default = "default_tenon_depth")]
    tenon_depth_mm: f32,
    /// "frustum" (default) or "dome".
    #[serde(default = "default_tenon_shape")]
    tenon_shape: String,
    /// Edge fillet radius in mm (rounds the frustum corners + tip).
    #[serde(default)]
    tenon_fillet_mm: f32,
    /// Tenon/mortise fit tolerance in mm — how much larger the mortise is carved
    /// on every face. 0 = press fit; 0.1 is a slide fit.
    #[serde(default = "default_tenon_tolerance")]
    tenon_tolerance_mm: f32,
    /// Where the tenon sits on the cut face: the model-local point the user put the
    /// crosshair on. Absent = the natural middle of the cut.
    #[serde(default)]
    tenon_anchor: Option<[f32; 3]>,
    /// Flat-cut only: the plane the tenon is framed on, as `dot(normal, p) ==
    /// offset` in model-local space — the exact plane the frontend previewed.
    #[serde(default)]
    plane_normal: [f32; 3],
    #[serde(default)]
    plane_offset: f32,
    /// Flip which half gets the tenon vs the mortise.
    #[serde(default)]
    tenon_swap_sides: bool,
    /// Tenon tilt (radians) — polar lean off the cut normal.
    #[serde(default)]
    tenon_tilt_rad: f32,
    /// Tenon roll (radians) — spin about the tenon's own axis.
    #[serde(default)]
    tenon_roll_rad: f32,
}

fn default_density_one() -> f32 {
    1.0
}
fn default_tenon_width() -> f32 {
    2.0
}
fn default_tenon_depth() -> f32 {
    2.5
}
fn default_tenon_shape() -> String {
    "frustum".to_string()
}
fn default_tenon_tolerance() -> f32 {
    0.1
}

impl Default for GeodesicRequestDto {
    fn default() -> Self {
        Self {
            points: Vec::new(),
            close: false,
            smoothing: 0.5,
            membrane_smoothing: 0.5,
            density: 1.0,
            joint_clearance_mm: 0.0,
            generate_tenon: false,
            tenon_width_mm: 2.0,
            tenon_depth_mm: 2.5,
            tenon_shape: "frustum".to_string(),
            tenon_fillet_mm: 0.0,
            tenon_tolerance_mm: 0.1,
            tenon_anchor: None,
            plane_normal: [0.0; 3],
            plane_offset: 0.0,
            tenon_swap_sides: false,
            tenon_tilt_rad: 0.0,
            tenon_roll_rad: 0.0,
        }
    }
}

fn parse_geodesic_request(json: &str) -> GeodesicRequestDto {
    if json.trim().is_empty() {
        return GeodesicRequestDto::default();
    }
    // A malformed preview request falls back to the defaults: the preview is live
    // and a typing user must not see an error for a half-built drag.
    serde_json::from_str::<GeodesicRequestDto>(json).unwrap_or_default()
}

fn parse_organic_cut_options(options_json: &str) -> OrganicCutOptions {
    if options_json.trim().is_empty() {
        return OrganicCutOptions::default();
    }
    serde_json::from_str::<OrganicCutOptions>(options_json).unwrap_or_default()
}

/// Minimal JSON string-body escaper for the short ASCII status messages embedded
/// in the hand-built preview objects (quotes + backslashes only).
fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Writes the EXACT inputs of an organic cut to disk when `DF_CUT_DUMP` names a
/// directory: the staged mesh as raw LE f32 soup (`cut-<n>.bin`) and the options
/// JSON (`cut-<n>.json`), numbered so a session keeps every attempt. The pair is a
/// complete, lossless replay of the cut — feed it to the `cut_replay` binary in
/// `dragonfruit-organic-cut` to reproduce a bad cut outside the app. Off unless the
/// variable is set; a write failure only warns (never breaks the cut).
fn dump_organic_cut_inputs(bytes: &[u8], options_json: &str) {
    let Some(dir) = std::env::var_os("DF_CUT_DUMP") else {
        return;
    };
    let dir = std::path::PathBuf::from(dir);
    if let Err(e) = std::fs::create_dir_all(&dir) {
        log::warn!("[cut] dump dir {}: {e}", dir.display());
        return;
    }
    let n = (0u32..)
        .find(|n| !dir.join(format!("cut-{n}.bin")).exists())
        .unwrap_or(0);
    if let Err(e) = std::fs::write(dir.join(format!("cut-{n}.bin")), bytes)
        .and_then(|()| std::fs::write(dir.join(format!("cut-{n}.json")), options_json))
    {
        log::warn!("[cut] dump cut-{n}: {e}");
        return;
    }
    log::info!(
        "[cut] dumped inputs to {}/cut-{n}.{{bin,json}} ({} bytes of mesh)",
        dir.display(),
        bytes.len()
    );
}

fn parts_to_soup(parts: &[IndexedMesh]) -> Vec<Vec<u8>> {
    parts
        .iter()
        .map(|p| bytemuck::cast_slice::<f32, u8>(&p.to_triangle_soup()).to_vec())
        .collect()
}

/// `mesh_organic_cut_staged`: cut the staged mesh, replacing the staged buffer with
/// the first part and stashing ALL parts for read-back.
///
/// A multi-loop cut can produce more than two parts. A no-op cut produces none, in
/// which case the staged mesh is left as it was.
pub fn mesh_organic_cut_staged(
    state: &OrganicCutState,
    staging: &MeshStaging,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_organic_cut_options(options_json);
    let bytes = staging.read_staging_bytes()?;
    dump_organic_cut_inputs(&bytes, options_json);

    let mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("organic cut task panicked: {e}"))?;
    let OrganicCutOutcome { parts, report } = organic_cut(mesh, &options);

    *OrganicCutState::lock(&state.parts_bytes, "organic cut parts")? = parts_to_soup(&parts);
    if let Some(first) = parts.first() {
        staging.replace_with_mesh(first)?;
    }
    serde_json::to_string(&report).map_err(|e| format!("serialize organic cut report: {e}"))
}

/// `mesh_organic_cut_capture_staged_source`: capture the staged mesh as the source
/// for repeated non-mutating cuts.
pub fn mesh_organic_cut_capture_staged_source(
    state: &OrganicCutState,
    staging: &MeshStaging,
) -> BackendResult<()> {
    let bytes = staging.read_staging_bytes()?;
    *OrganicCutState::lock(&state.source_bytes, "organic cut source")? = Some(bytes);
    OrganicCutState::lock(&state.parts_bytes, "organic cut parts")?.clear();
    // The cached solver belongs to the previous source; the next geodesic call
    // rebuilds it lazily for the new mesh.
    *OrganicCutState::lock(&state.geodesic_solver, "organic cut solver")? = None;
    Ok(())
}

/// `mesh_organic_cut_from_captured_source`: cut the captured source without
/// mutating the staged mesh. Stashes all parts, driven by the report's `partCount`.
pub fn mesh_organic_cut_from_captured_source(
    state: &OrganicCutState,
    options_json: &str,
) -> BackendResult<String> {
    let options = parse_organic_cut_options(options_json);
    let source_bytes = OrganicCutState::lock(&state.source_bytes, "organic cut source")?
        .clone()
        .ok_or_else(|| {
            "No captured organic cut source — call mesh_organic_cut_capture_staged_source first"
                .to_string()
        })?;
    dump_organic_cut_inputs(&source_bytes, options_json);

    let mesh = io::staged::load_positions_le(&source_bytes)
        .map_err(|e| format!("organic cut task panicked: {e}"))?;
    let OrganicCutOutcome { parts, report } = organic_cut(mesh, &options);

    *OrganicCutState::lock(&state.parts_bytes, "organic cut parts")? = parts_to_soup(&parts);
    serde_json::to_string(&report).map_err(|e| format!("serialize organic cut report: {e}"))
}

/// `mesh_organic_cut_read_part`: part `index` of the last cut, as raw LE f32 soup.
///
/// Indices run `0..report.partCount`; out-of-range indices error.
pub fn mesh_organic_cut_read_part(state: &OrganicCutState, index: usize) -> BackendResult<Vec<u8>> {
    OrganicCutState::lock(&state.parts_bytes, "organic cut parts")?
        .get(index)
        .cloned()
        .ok_or_else(|| format!("No organic cut part at index {index} — run a cut first"))
}

/// The solver for the captured source, built once and cached.
fn geodesic_solver_or_build(state: &OrganicCutState) -> BackendResult<Arc<GeodesicSolver>> {
    if let Some(solver) = OrganicCutState::lock(&state.geodesic_solver, "organic cut solver")?
        .clone()
    {
        return Ok(solver);
    }
    let source_bytes = OrganicCutState::lock(&state.source_bytes, "organic cut source")?
        .clone()
        .ok_or_else(|| {
            "No captured source — call mesh_organic_cut_capture_staged_source first".to_string()
        })?;
    let mesh = io::staged::load_positions_le(&source_bytes)
        .map_err(|e| format!("geodesic solver build panicked: {e}"))?;
    let built = Arc::new(GeodesicSolver::build(mesh));
    *OrganicCutState::lock(&state.geodesic_solver, "organic cut solver")? = Some(built.clone());
    Ok(built)
}

/// The flat LE f32 polyline bytes plus its point count, for a validated request.
fn compute_geodesic_loop_bytes(
    state: &OrganicCutState,
    req: &GeodesicRequestDto,
) -> BackendResult<(Vec<u8>, usize)> {
    let solver = geodesic_solver_or_build(state)?;
    let waypoints: Vec<Vec3> = req
        .points
        .iter()
        .map(|p| Vec3::new(p.position[0], p.position[1], p.position[2]))
        .collect();
    let loop_pts = solver
        .surface_loop_smoothed(&waypoints, req.close, req.smoothing)
        .ok_or_else(|| "geodesic loop could not be computed".to_string())?;
    let flat: Vec<f32> = loop_pts.iter().flat_map(|v| [v.x, v.y, v.z]).collect();
    let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&flat).to_vec();
    Ok((bytes, loop_pts.len()))
}

/// `mesh_organic_cut_geodesic_loop`: compute, stash and describe the on-surface
/// seam through the given waypoints.
///
/// Fewer than two waypoints clears any stale polyline and reports `pointCount: 0`
/// — that is the state a drag starts in, not an error.
pub fn mesh_organic_cut_geodesic_loop(
    state: &OrganicCutState,
    request_json: &str,
) -> BackendResult<String> {
    let req = parse_geodesic_request(request_json);
    if req.points.len() < 2 {
        *OrganicCutState::lock(&state.geodesic_bytes, "geodesic")? = None;
        return Ok("{\"pointCount\":0}".to_string());
    }
    let (bytes, count) = compute_geodesic_loop_bytes(state, &req)?;
    *OrganicCutState::lock(&state.geodesic_bytes, "geodesic")? = Some(bytes);
    Ok(format!("{{\"pointCount\":{count}}}"))
}

/// `mesh_organic_cut_geodesic_loop_bytes`: the same loop, returned as raw bytes
/// instead of being stashed first.
///
/// This is the hot path while dragging a waypoint — one round trip per frame
/// instead of two. It also stashes the bytes so a later
/// [`mesh_organic_cut_read_geodesic`] still works.
pub fn mesh_organic_cut_geodesic_loop_bytes(
    state: &OrganicCutState,
    request_json: &str,
) -> BackendResult<Vec<u8>> {
    let req = parse_geodesic_request(request_json);
    if req.points.len() < 2 {
        *OrganicCutState::lock(&state.geodesic_bytes, "geodesic")? = None;
        return Ok(Vec::new());
    }
    let (bytes, _count) = compute_geodesic_loop_bytes(state, &req)?;
    *OrganicCutState::lock(&state.geodesic_bytes, "geodesic")? = Some(bytes.clone());
    Ok(bytes)
}

/// `mesh_organic_cut_read_geodesic`: the last loop polyline, raw LE f32 (3 floats
/// per point). Empty when nothing has been computed.
pub fn mesh_organic_cut_read_geodesic(state: &OrganicCutState) -> BackendResult<Vec<u8>> {
    Ok(
        OrganicCutState::lock(&state.geodesic_bytes, "geodesic")?
            .clone()
            .unwrap_or_default(),
    )
}

/// The placement-frame JSON the gizmo reads, in model-local coordinates.
fn tenon_frame_json(frame: Option<dragonfruit_organic_cut::TenonFrameInfo>) -> String {
    match frame {
        Some(f) => format!(
            "{{\"anchor\":[{},{},{}],\"axis\":[{},{},{}],\"u\":[{},{},{}],\"v\":[{},{},{}],\"tip\":[{},{},{}],\"depth\":{},\"maxTiltRad\":{},\"halfDiagMm\":{}}}",
            f.anchor.x, f.anchor.y, f.anchor.z,
            f.axis.x, f.axis.y, f.axis.z,
            f.u.x, f.u.y, f.u.z,
            f.v.x, f.v.y, f.v.z,
            f.tip.x, f.tip.y, f.tip.z,
            f.depth,
            f.max_tilt,
            f.half_diag,
        ),
        None => "null".to_string(),
    }
}

/// `mesh_organic_cut_membrane_preview`: build the contour-cut cutter surface for
/// the loop and stash it as a triangle soup for previewing.
///
/// Uses the same loop the cut would (the dense geodesic points the caller passes),
/// so what is rendered IS the cutter. With no captured source it falls back to a
/// bare membrane on the raw loop, which cannot apply the real offset but still
/// previews the shape.
pub fn mesh_organic_cut_membrane_preview(
    state: &OrganicCutState,
    request_json: &str,
) -> BackendResult<String> {
    let req = parse_geodesic_request(request_json);
    if req.points.len() < 3 {
        *OrganicCutState::lock(&state.membrane_bytes, "membrane")? = None;
        return Ok("{\"triangleCount\":0}".to_string());
    }

    let loop_pts: Vec<Vec3> = req
        .points
        .iter()
        .map(|p| Vec3::new(p.position[0], p.position[1], p.position[2]))
        .collect();

    // The cutter preview draws the wafer, whose thickness is no longer a setting —
    // it is sub-resolution either way, and the surface cut it stands behind has no
    // thickness at all.
    let thickness_mm = dragonfruit_organic_cut::membrane::DEFAULT_CUTTER_THICKNESS_MM;
    let joint_clearance_mm = req.joint_clearance_mm.max(0.0);
    let tenon_shape = TenonShape::from_str_or_default(&req.tenon_shape);
    let tenon_at = req.tenon_anchor.map(|p| Vec3::new(p[0], p[1], p[2]));
    let tenon_tilt = TenonTilt::new(req.tenon_tilt_rad, req.tenon_roll_rad);

    let source_bytes = OrganicCutState::lock(&state.source_bytes, "organic cut source")?.clone();

    // The tenon preview needs the real source mesh (to probe wall clearance); it is
    // only available once the source is captured. An empty soup means no tenon — too
    // thin, degenerate, or no source.
    let mut tenon_soup: Vec<f32> = Vec::new();
    let mut tenon_tris = 0usize;
    let mut tenon_kind = "none".to_string();
    let mut tenon_detail = String::new();
    // Whether the previewed tenon can actually be placed. A tenon that cannot is
    // still built and still framed — drawn in the won't-fit colour, with its gizmo —
    // so `false` means "show it in red and refuse the cut", not "no tenon". Defaults
    // true so a cut with no tenon requested never blocks.
    let mut tenon_fits = true;
    let mut tenon_frame = None;

    let soup = if let Some(bytes) = source_bytes {
        let mesh = io::staged::load_positions_le(&bytes)
            .map_err(|e| format!("membrane preview task panicked: {e}"))?;
        if req.generate_tenon {
            if let Some(preview) = dragonfruit_organic_cut::build_tenon_preview_soup(
                &mesh,
                &loop_pts,
                req.membrane_smoothing,
                req.density,
                tenon_shape,
                req.tenon_swap_sides,
                tenon_tilt,
                req.tenon_width_mm,
                req.tenon_depth_mm,
                req.tenon_fillet_mm,
                // The same sum the cut spends, so the previewed fit is the real one.
                req.tenon_tolerance_mm + joint_clearance_mm,
                tenon_at,
            ) {
                tenon_tris = preview.tenon_triangles;
                tenon_soup = preview.soup;
                tenon_kind = preview.kind.as_str().to_string();
                tenon_fits = preview.fits;
                tenon_detail = preview.detail;
                tenon_frame = preview.frame;
            }
        }
        dragonfruit_organic_cut::membrane::build_cutter_preview_soup(
            &mesh,
            &loop_pts,
            thickness_mm,
            req.membrane_smoothing,
            req.density,
        )
        .ok_or_else(|| "cutter could not be built from the loop".to_string())?
    } else {
        dragonfruit_organic_cut::membrane::build_membrane_preview_soup_full(
            &loop_pts,
            req.membrane_smoothing,
            req.density,
        )
        .ok_or_else(|| "membrane could not be built from the loop".to_string())?
    };

    let tri_count = soup.len() / 9;
    let joint_tris = tenon_soup.len() / 9;
    *OrganicCutState::lock(&state.membrane_bytes, "membrane")? =
        Some(bytemuck::cast_slice::<f32, u8>(&soup).to_vec());
    *OrganicCutState::lock(&state.tenon_bytes, "tenon")? =
        Some(bytemuck::cast_slice::<f32, u8>(&tenon_soup).to_vec());

    Ok(format!(
        "{{\"triangleCount\":{tri_count},\"jointTriangleCount\":{joint_tris},\"tenonTriangleCount\":{tenon_tris},\"tenonKind\":\"{tenon_kind}\",\"tenonFits\":{tenon_fits},\"tenonDetail\":\"{}\",\"tenonFrame\":{}}}",
        json_escape(&tenon_detail),
        tenon_frame_json(tenon_frame),
    ))
}

/// `mesh_organic_cut_plane_tenon_preview`: the tenon a FLAT cut would place, framed
/// on the cut plane.
///
/// A contour preview builds a membrane from the loop and frames the tenon on it; a
/// flat cut has no membrane, so this frames the tenon on the cross-section the plane
/// carves. Same ladder and same build as the cut, so the preview is what lands.
/// `triangleCount` is 0 because there is no membrane of its own.
pub fn mesh_organic_cut_plane_tenon_preview(
    state: &OrganicCutState,
    request_json: &str,
) -> BackendResult<String> {
    let req = parse_geodesic_request(request_json);
    let normal = Vec3::new(req.plane_normal[0], req.plane_normal[1], req.plane_normal[2]);
    let tenon_shape = TenonShape::from_str_or_default(&req.tenon_shape);
    let tenon_tilt = TenonTilt::new(req.tenon_tilt_rad, req.tenon_roll_rad);
    let tenon_at = req.tenon_anchor.map(|p| Vec3::new(p[0], p[1], p[2]));

    let source_bytes = OrganicCutState::lock(&state.source_bytes, "organic cut source")?.clone();
    let Some(bytes) = source_bytes else {
        return Ok(NO_TENON_PREVIEW_JSON.to_string());
    };
    if !req.generate_tenon || normal.length() < 1e-6 {
        return Ok(NO_TENON_PREVIEW_JSON.to_string());
    }

    let mesh = io::staged::load_positions_le(&bytes)
        .map_err(|e| format!("plane tenon preview task panicked: {e}"))?;
    let Some(frame) = dragonfruit_organic_cut::frame_from_plane(
        &mesh,
        normal,
        req.plane_offset,
        tenon_at,
    ) else {
        return Ok(NO_TENON_PREVIEW_JSON.to_string());
    };
    let preview = dragonfruit_organic_cut::build_tenon_preview_at_frame(
        &mesh,
        frame,
        tenon_shape,
        req.tenon_swap_sides,
        tenon_tilt,
        req.tenon_width_mm,
        req.tenon_depth_mm,
        req.tenon_fillet_mm,
        req.tenon_tolerance_mm,
    );

    let joint_tris = preview.soup.len() / 9;
    *OrganicCutState::lock(&state.tenon_bytes, "tenon")? =
        Some(bytemuck::cast_slice::<f32, u8>(&preview.soup).to_vec());

    Ok(format!(
        "{{\"triangleCount\":0,\"jointTriangleCount\":{joint_tris},\"tenonTriangleCount\":{},\"tenonKind\":\"{}\",\"tenonFits\":{},\"tenonDetail\":\"{}\",\"tenonFrame\":{}}}",
        preview.tenon_triangles,
        preview.kind.as_str(),
        preview.fits,
        json_escape(&preview.detail),
        tenon_frame_json(preview.frame),
    ))
}

/// `mesh_organic_cut_read_tenon`: the last registration-tenon preview, raw LE f32
/// soup (tenon followed by mortise). Empty when no tenon was previewed.
pub fn mesh_organic_cut_read_tenon(state: &OrganicCutState) -> BackendResult<Vec<u8>> {
    Ok(
        OrganicCutState::lock(&state.tenon_bytes, "tenon")?
            .clone()
            .unwrap_or_default(),
    )
}

/// `mesh_organic_cut_read_membrane`: the last membrane preview, raw LE f32 soup.
pub fn mesh_organic_cut_read_membrane(state: &OrganicCutState) -> BackendResult<Vec<u8>> {
    Ok(
        OrganicCutState::lock(&state.membrane_bytes, "membrane")?
            .clone()
            .unwrap_or_default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

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

    fn staged_cube() -> MeshStaging {
        let soup = cube_soup();
        let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();
        let staging = MeshStaging::new();
        staging.stage_mesh_binary_set(&bytes).expect("stage");
        staging
    }

    #[test]
    fn a_preview_request_with_no_points_is_not_an_error() {
        let state = OrganicCutState::new();
        assert_eq!(
            mesh_organic_cut_geodesic_loop(&state, "").expect("loop"),
            "{\"pointCount\":0}"
        );
        assert!(mesh_organic_cut_geodesic_loop_bytes(&state, "{\"points\":[]}")
            .expect("bytes")
            .is_empty());
        assert_eq!(
            mesh_organic_cut_membrane_preview(&state, "").expect("membrane"),
            "{\"triangleCount\":0}"
        );
    }

    #[test]
    fn a_malformed_request_falls_back_to_the_defaults() {
        let state = OrganicCutState::new();
        // Not valid JSON at all: the live preview keeps working on defaults.
        assert_eq!(
            mesh_organic_cut_geodesic_loop(&state, "{not json")
                .expect("loop"),
            "{\"pointCount\":0}"
        );
    }

    #[test]
    fn capture_resets_parts_and_the_cached_solver() {
        let staging = staged_cube();
        let state = OrganicCutState::new();
        mesh_organic_cut_capture_staged_source(&state, &staging).expect("capture");
        assert!(mesh_organic_cut_read_geodesic(&state).expect("geodesic").is_empty());
        assert!(mesh_organic_cut_read_tenon(&state).expect("tenon").is_empty());
        assert!(mesh_organic_cut_read_membrane(&state).expect("membrane").is_empty());
        assert!(mesh_organic_cut_read_part(&state, 0).is_err());
    }

    #[test]
    fn a_flat_tenon_preview_without_a_source_answers_the_empty_shape() {
        let state = OrganicCutState::new();
        let request = "{\"generateTenon\":true,\"planeNormal\":[0,0,1],\"planeOffset\":5}";
        assert_eq!(
            mesh_organic_cut_plane_tenon_preview(&state, request).expect("preview"),
            NO_TENON_PREVIEW_JSON
        );
    }

    #[test]
    fn a_flat_tenon_preview_needs_a_tenon_and_a_normal() {
        let staging = staged_cube();
        let state = OrganicCutState::new();
        mesh_organic_cut_capture_staged_source(&state, &staging).expect("capture");
        // generateTenon off.
        assert_eq!(
            mesh_organic_cut_plane_tenon_preview(&state, "{\"planeNormal\":[0,0,1]}")
                .expect("preview"),
            NO_TENON_PREVIEW_JSON
        );
        // A degenerate normal has no plane to frame.
        assert_eq!(
            mesh_organic_cut_plane_tenon_preview(&state, "{\"generateTenon\":true}")
                .expect("preview"),
            NO_TENON_PREVIEW_JSON
        );
    }
}
