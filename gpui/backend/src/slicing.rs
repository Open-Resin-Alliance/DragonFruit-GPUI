//! Slicing — `slice_solid_native_to_temp_path`, `cancel_slicing`.
//!
//! Ported from `src-tauri/src/main.rs` lines 750-1000 and 1578-1830. The slice is
//! `dragonfruit-slicing-engine::engine::slice_with_progress_v3_to_path`; what lived
//! in the shell was job-metadata parsing (the frontend sent a JSON object and a
//! separate binary mesh), the mesh decode, the progress throttle, and the perf
//! counters the UI shows.
//!
//! ## Two things kept on purpose
//!
//! * **The named rayon pool.** Thread count and thread names are a measured
//!   performance contract (`dragonfruit-slicing-engine-<n>` shows up in every
//!   profile), so the pool stays process-wide and named rather than joining
//!   whatever pool the app has.
//! * **The 8 ms progress throttle.** A slice reports per layer; emitting every one
//!   at 4k layers would cost more than the slice. The first event, any phase change
//!   and the final event always go; intermediate ones are dropped inside an 8 ms
//!   window.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use dragonfruit_slicing_engine::types::{
    ProgressCallbackV3, SliceProgressPhaseV3, SliceProgressUpdateV3,
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use serde::{Deserialize, Serialize};

use crate::staging::MeshStaging;
use crate::BackendResult;

/// The export format used when the job names none. `src-tauri` asked the plugin
/// registry's `FormatProvider`, whose built-in answer is `"print"`
/// (`plugin_registry.rs`, `DefaultFormatProvider::default_export_format`); a build
/// that registers a different provider passes the format in the job instead.
pub const DEFAULT_EXPORT_FORMAT: &str = "print";

/// Progress payload pushed to the UI. Event name: `slicer://progress`.
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SliceProgressPayload {
    pub done: u32,
    pub total: u32,
    pub phase: String,
}

/// The UI's progress callback. `Arc` rather than a borrowed trait object because
/// the engine's own `ProgressCallbackV3` is `'static`.
pub type SliceSink = Arc<dyn Fn(SliceProgressPayload) + Send + Sync>;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSlicerPerfMetrics {
    pub total_ns: u64,
    pub index_build_ns: u64,
    pub render_wall_ns: u64,
    pub render_ns: u64,
    pub png_encode_ns: u64,
    pub archive_encode_ns: u64,
    pub z_blend_backward_ns: u64,
    pub z_blend_forward_ns: u64,
    pub cross_blend_ns: u64,
    pub cross_blend_touched_pixels: u64,
    pub cross_blend_contributing_layers: u64,
    pub post_blur_ns: u64,
    pub support_merge_ns: u64,
    pub layers: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSlicerRuntimeMetrics {
    pub pool_threads: u32,
    pub max_concurrent: u32,
    pub queue_buffer: u32,
    pub daa_post_threads: u32,
    pub daa_post_buffer_depth: u32,
    pub build_profile: String,
    pub artifact_dir: String,
    pub mesh_stage_dir: String,
    pub metadata_parse_ns: u64,
    pub mesh_decode_ns: u64,
    pub artifact_metadata_ns: u64,
    pub wrapper_total_ns: u64,
    pub wrapper_overhead_ns: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeSliceTempPathResult {
    pub temp_path: String,
    pub byte_len: u64,
    pub perf: Option<NativeSlicerPerfMetrics>,
    pub runtime: NativeSlicerRuntimeMetrics,
}

// ---------------------------------------------------------------------------
// Job metadata
// ---------------------------------------------------------------------------

/// Bounding box of a `quantized_u16` mesh, for decoding back to millimetres.
#[derive(Deserialize)]
struct MeshQuantizationMetadata {
    min_x: f32,
    min_y: f32,
    min_z: f32,
    max_x: f32,
    max_y: f32,
    max_z: f32,
}

fn default_minimum_aa_alpha_percent() -> f32 {
    35.0
}
fn default_blur_brush_radius_px() -> u32 {
    1
}
fn default_blur_brush_kernel() -> String {
    "gaussian".to_string()
}
fn default_blur_brush_sigma_x() -> f64 {
    0.5
}
fn default_blur_brush_sigma_y() -> f64 {
    0.5
}
fn default_z_blur_radius_layers() -> u32 {
    0
}
fn default_z_blur_kernel() -> String {
    "box".to_string()
}
fn default_z_blur_sigma() -> f64 {
    0.5
}
fn default_tank_count() -> u32 {
    1
}
fn default_anti_aliasing_mode() -> String {
    "Blur".to_string()
}
fn default_z_blend_look_back() -> u32 {
    2
}
fn default_z_blend_max_alpha_percent() -> f32 {
    90.0
}
fn default_dither_device_gamma() -> f64 {
    3.0
}

/// A slice job as the frontend described it: settings plus a reference to the
/// staged mesh (the triangles themselves arrive through [`MeshStaging`]).
///
/// The `#[serde(default = …)]` functions are the frontend's own defaults; they are
/// repeated here rather than in the engine because the engine's job type has no
/// serde contract — it is a value the UI builds.
#[derive(Deserialize)]
struct SliceJobMetadata {
    output_format: String,
    #[serde(default)]
    format_version: Option<String>,
    #[serde(default)]
    output_path: Option<String>,
    source_width_px: u32,
    source_height_px: u32,
    width_px: u32,
    height_px: u32,
    #[serde(default)]
    x_packing_mode: Option<String>,
    png_compression_strategy: String,
    anti_aliasing_level: String,
    #[serde(default = "default_anti_aliasing_mode")]
    anti_aliasing_mode: String,
    #[serde(default = "default_blur_brush_radius_px")]
    blur_brush_radius_px: u32,
    #[serde(default = "default_blur_brush_kernel")]
    blur_brush_kernel: String,
    #[serde(default = "default_blur_brush_sigma_x", alias = "blur_brush_sigma")]
    blur_brush_sigma_x: f64,
    #[serde(default = "default_blur_brush_sigma_y")]
    blur_brush_sigma_y: f64,
    #[serde(default = "default_z_blur_radius_layers")]
    z_blur_radius_layers: u32,
    #[serde(default = "default_z_blur_kernel")]
    z_blur_kernel: String,
    #[serde(default = "default_z_blur_sigma")]
    z_blur_sigma: f64,
    aa_on_supports: bool,
    #[serde(default = "default_minimum_aa_alpha_percent")]
    minimum_aa_alpha_percent: f32,
    #[serde(default)]
    mirror_x: bool,
    #[serde(default)]
    mirror_y: bool,
    #[serde(default = "default_z_blend_look_back")]
    z_blend_look_back: u32,
    #[serde(default)]
    z_blend_minimum_alpha_percent: f32,
    /// Maximum gray level (0-100 %) for z-blend gradient pixels at the inner boundary.
    #[serde(default = "default_z_blend_max_alpha_percent")]
    z_blend_max_alpha_percent: f32,
    /// Optional custom grayscale cure LUT (256 u8 values).
    ///
    /// Used by 3DAA to override the linear cure-window ramp and by 2D Blur AA to
    /// remap the post-blur grayscale output directly.
    #[serde(default)]
    z_blend_custom_lut: Option<Vec<u8>>,
    #[serde(default)]
    zaa_kernel: Option<String>,
    #[serde(default)]
    zaa_pattern: Option<String>,
    #[serde(default)]
    zaa_duplicate_z: Option<bool>,
    #[serde(default)]
    model_triangle_count: u32,
    /// Resin tanks the job prints; `1` for every single-material machine.
    #[serde(default = "default_tank_count")]
    tank_count: u32,
    /// Tank per triangle, aligned with the mesh. Empty means all Tank 1.
    #[serde(default)]
    triangle_tank_index: Vec<u32>,
    container_compression_level: u8,
    build_width_mm: f32,
    build_depth_mm: f32,
    layer_height_mm: f32,
    total_layers: u32,
    export_thumbnail_png_base64: Option<String>,
    #[serde(default)]
    dither_enabled: bool,
    #[serde(default)]
    dither_bit_depth: Option<u32>,
    #[serde(default = "default_dither_device_gamma")]
    dither_device_gamma: f64,
    #[serde(default)]
    mesh_encoding: Option<String>,
    #[serde(default)]
    mesh_quantization: Option<MeshQuantizationMetadata>,
    metadata_json: String,
}

// ---------------------------------------------------------------------------
// Mesh decoding
// ---------------------------------------------------------------------------

/// Decode a staged soup that is raw little-endian `f32`s.
///
/// On a little-endian target this memcpys into a pre-sized `Vec<f32>` rather than
/// converting float by float; the mesh arrives as millions of them.
pub fn bytes_to_f32_soup(bytes: &[u8]) -> BackendResult<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return Err(format!(
            "Mesh binary length {} is not a multiple of 4 (f32 size)",
            bytes.len()
        ));
    }
    let count = bytes.len() / 4;

    #[cfg(target_endian = "little")]
    {
        let mut floats = vec![0.0f32; count];
        // SAFETY: equal-length, non-overlapping, and `floats` is exactly
        // `bytes.len()` bytes; u8 has no alignment requirement.
        unsafe {
            std::ptr::copy_nonoverlapping(
                bytes.as_ptr(),
                floats.as_mut_ptr() as *mut u8,
                bytes.len(),
            );
        }
        return Ok(floats);
    }

    #[cfg(not(target_endian = "little"))]
    {
        let mut floats = Vec::with_capacity(count);
        for chunk in bytes.chunks_exact(4) {
            floats.push(f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]));
        }
        Ok(floats)
    }
}

/// [`bytes_to_f32_soup`] for a buffer the caller no longer needs: reinterprets it
/// in place when alignment and capacity allow, rather than copying.
pub fn bytes_into_f32_soup(bytes: Vec<u8>) -> BackendResult<Vec<f32>> {
    if bytes.len() % 4 != 0 {
        return Err(format!(
            "Mesh binary length {} is not a multiple of 4 (f32 size)",
            bytes.len()
        ));
    }

    #[cfg(target_endian = "little")]
    {
        let ptr_addr = bytes.as_ptr() as usize;
        let aligned_for_f32 = ptr_addr % std::mem::align_of::<f32>() == 0;
        let capacity_bytes = bytes.capacity();

        if aligned_for_f32 && (capacity_bytes % 4 == 0) {
            let mut bytes = std::mem::ManuallyDrop::new(bytes);
            let ptr = bytes.as_mut_ptr() as *mut f32;
            let len_f32 = bytes.len() / 4;
            let cap_f32 = bytes.capacity() / 4;
            // SAFETY: the layout matches (`f32` is 4 bytes and the pointer is
            // aligned), the allocation came from the global allocator with the same
            // alignment, and `ManuallyDrop` stops the byte buffer from freeing it.
            return Ok(unsafe { Vec::from_raw_parts(ptr, len_f32, cap_f32) });
        }
        return bytes_to_f32_soup(&bytes);
    }

    #[cfg(not(target_endian = "little"))]
    {
        bytes_to_f32_soup(&bytes)
    }
}

/// Decode a `quantized_u16` mesh: one `u16` per coordinate, mapped back through the
/// job's bounding box.
fn decode_quantized_u16_mesh(
    bytes: &[u8],
    quant: &MeshQuantizationMetadata,
) -> BackendResult<Vec<f32>> {
    if bytes.len() % 2 != 0 {
        return Err(format!(
            "Quantized mesh binary length {} is not a multiple of 2 (u16 size)",
            bytes.len()
        ));
    }

    let spans = [
        (quant.max_x - quant.min_x).max(0.0),
        (quant.max_y - quant.min_y).max(0.0),
        (quant.max_z - quant.min_z).max(0.0),
    ];
    let mins = [quant.min_x, quant.min_y, quant.min_z];
    let values = bytes.len() / 2;
    let mut floats = Vec::with_capacity(values);

    for (index, chunk) in bytes.chunks_exact(2).enumerate() {
        let q = u16::from_le_bytes([chunk[0], chunk[1]]);
        let axis = index % 3;
        let span = spans[axis];
        if span <= 0.0 || !span.is_finite() {
            floats.push(mins[axis]);
            continue;
        }
        let normalized = (q as f32) / 65_535.0;
        floats.push(mins[axis] + normalized * span);
    }
    Ok(floats)
}

fn decode_mesh_bytes(bytes: Vec<u8>, meta: &SliceJobMetadata) -> BackendResult<Vec<f32>> {
    match meta.mesh_encoding.as_deref() {
        Some("quantized_u16") => {
            let quant = meta
                .mesh_quantization
                .as_ref()
                .ok_or("Quantized mesh encoding requires mesh_quantization metadata")?;
            decode_quantized_u16_mesh(&bytes, quant)
        }
        Some("raw_f32") | None => bytes_into_f32_soup(bytes),
        Some(other) => Err(format!("Unsupported mesh encoding: {other}")),
    }
}

// ---------------------------------------------------------------------------
// Progress and pool
// ---------------------------------------------------------------------------

fn duration_ns_u64(duration: std::time::Duration) -> u64 {
    duration.as_nanos().min(u64::MAX as u128) as u64
}

fn phase_to_label(phase: SliceProgressPhaseV3) -> &'static str {
    match phase {
        SliceProgressPhaseV3::Slicing => "Slicing",
        SliceProgressPhaseV3::Encoding => "Encoding",
        SliceProgressPhaseV3::Finalizing => "Finalizing",
    }
}

/// The slicer's own pool, kept for its measured thread count and thread names.
///
/// Shared with [`crate::islands`]: the island scan has the same shape of work (a
/// layer-at-a-time raster walk) and the shell ran it on this pool too.
pub(crate) fn slicer_pool() -> &'static ThreadPool {
    static SLICER_POOL: std::sync::LazyLock<ThreadPool> = std::sync::LazyLock::new(|| {
        let threads = std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1);
        ThreadPoolBuilder::new()
            .thread_name(|i| format!("dragonfruit-slicing-engine-{i}"))
            .num_threads(threads)
            .build()
            .expect("failed to create slicer rayon thread pool")
    });
    &SLICER_POOL
}

/// Throttle the engine's per-layer updates to ~60 fps.
///
/// The first event, any phase change and the final event (`done == total`) are
/// always delivered; intermediate ones are skipped inside an 8 ms window. The state
/// is behind a `Mutex` because the engine calls the callback from its own threads.
fn make_throttled_progress_cb(sink: Option<SliceSink>) -> Option<ProgressCallbackV3> {
    use std::time::Instant;

    let sink = sink?;

    struct State {
        last_emit: Instant,
        last_phase: String,
    }

    let state = Arc::new(Mutex::new(State {
        last_emit: Instant::now(),
        last_phase: String::new(),
    }));

    Some(Arc::new(move |update: SliceProgressUpdateV3| {
        let phase = phase_to_label(update.phase).to_string();
        let is_final = update.done >= update.total;

        let should_emit = {
            let Ok(mut s) = state.lock() else {
                // A poisoned throttle must not silence progress for good.
                return sink(SliceProgressPayload {
                    done: update.done,
                    total: update.total,
                    phase,
                });
            };
            let phase_changed = s.last_phase != phase;
            let elapsed = s.last_emit.elapsed().as_millis() >= 8;
            if is_final || phase_changed || elapsed {
                s.last_emit = Instant::now();
                s.last_phase = phase.clone();
                true
            } else {
                false
            }
        };

        if should_emit {
            sink(SliceProgressPayload {
                done: update.done,
                total: update.total,
                phase,
            });
        }
    }))
}

fn v3_runtime_metrics(
    output_path: &std::path::Path,
    daa_post_threads: u32,
    daa_post_buffer_depth: u32,
    metadata_parse_ns: u64,
    mesh_decode_ns: u64,
    artifact_metadata_ns: u64,
    wrapper_total_ns: u64,
    wrapper_overhead_ns: u64,
) -> NativeSlicerRuntimeMetrics {
    let hw_threads = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let max_concurrent = std::env::var("DF_V3_MAX_CONCURRENT")
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v >= 1)
        .unwrap_or(hw_threads)
        .clamp(1, hw_threads);
    let queue_buffer = (max_concurrent * 2).clamp(2, 16);

    let artifact_dir = output_path
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| std::env::temp_dir().to_string_lossy().to_string());
    let mesh_stage_dir = crate::staging::resolve_mesh_stage_directory()
        .to_string_lossy()
        .to_string();
    let build_profile = if cfg!(debug_assertions) {
        "debug".to_string()
    } else {
        "release".to_string()
    };

    NativeSlicerRuntimeMetrics {
        pool_threads: hw_threads as u32,
        max_concurrent: max_concurrent as u32,
        queue_buffer: queue_buffer as u32,
        daa_post_threads,
        daa_post_buffer_depth,
        build_profile,
        artifact_dir,
        mesh_stage_dir,
        metadata_parse_ns,
        mesh_decode_ns,
        artifact_metadata_ns,
        wrapper_total_ns,
        wrapper_overhead_ns,
    }
}

// ---------------------------------------------------------------------------
// The slice
// ---------------------------------------------------------------------------

/// The slicer's cancel flag, owned by whoever runs slices.
///
/// The shell kept one process-wide `Arc<AtomicBool>`; one per app (or per window)
/// is strictly better and costs nothing. A slice resets the flag when it starts, so
/// a cancel that arrives after the previous run finished cannot kill the next one.
pub struct Slicer {
    cancel: Arc<AtomicBool>,
}

impl Default for Slicer {
    fn default() -> Self {
        Self::new()
    }
}

impl Slicer {
    pub fn new() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    /// `cancel_slicing`: ask the running slice to stop. It checks the flag between
    /// layers, so this returns immediately and the slice unwinds from its own loop.
    pub fn cancel_slicing(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.cancel.load(Ordering::SeqCst)
    }

    /// `slice_solid_native_to_temp_path`: slice the staged mesh into a file.
    ///
    /// The staged soup is **taken**: a slice consumes its source, and the file
    /// variant can be gigabytes. The output goes to `outputPath` from the job when
    /// present, otherwise to a fresh [`crate::files::temp_artifact_path`] in the
    /// system temp directory named with the job's format.
    ///
    /// Progress event: `slicer://progress`, throttled by
    /// [`make_throttled_progress_cb`].
    pub fn slice_solid_native_to_temp_path(
        &self,
        staging: &MeshStaging,
        job_json: &str,
        sink: Option<SliceSink>,
    ) -> BackendResult<NativeSliceTempPathResult> {
        let mesh_bytes = staging.take_staged_bytes_for_slicing()?;

        self.cancel.store(false, Ordering::SeqCst);
        let flag = Arc::clone(&self.cancel);

        let wrapper_start = std::time::Instant::now();

        let metadata_parse_start = std::time::Instant::now();
        let meta: SliceJobMetadata = serde_json::from_str(job_json)
            .map_err(|err| format!("Invalid slice job metadata JSON: {err}"))?;
        let metadata_parse_ns = duration_ns_u64(metadata_parse_start.elapsed());
        let requested_output_path = meta
            .output_path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string());

        let mesh_decode_start = std::time::Instant::now();
        let triangles_xyz = decode_mesh_bytes(mesh_bytes, &meta)?;
        let mesh_decode_ns = duration_ns_u64(mesh_decode_start.elapsed());

        let job = dragonfruit_slicing_engine::SliceJobV3 {
            output_format: meta.output_format,
            format_version: meta.format_version,
            source_width_px: meta.source_width_px,
            source_height_px: meta.source_height_px,
            width_px: meta.width_px,
            height_px: meta.height_px,
            x_packing_mode: meta.x_packing_mode.unwrap_or_else(|| "none".to_string()),
            png_compression_strategy: meta.png_compression_strategy,
            anti_aliasing_level: meta.anti_aliasing_level,
            anti_aliasing_mode: meta.anti_aliasing_mode,
            blur_brush_radius_px: meta.blur_brush_radius_px,
            blur_brush_kernel: meta.blur_brush_kernel,
            blur_brush_sigma_x: meta.blur_brush_sigma_x,
            blur_brush_sigma_y: meta.blur_brush_sigma_y,
            z_blur_radius_layers: meta.z_blur_radius_layers,
            z_blur_kernel: meta.z_blur_kernel,
            z_blur_sigma: meta.z_blur_sigma,
            aa_on_supports: meta.aa_on_supports,
            model_triangle_count: meta.model_triangle_count,
            tank_count: meta.tank_count,
            triangle_tank_index: meta.triangle_tank_index,
            minimum_aa_alpha_percent: meta.minimum_aa_alpha_percent,
            mirror_x: meta.mirror_x,
            mirror_y: meta.mirror_y,
            z_blend_look_back: meta.z_blend_look_back,
            z_blend_minimum_alpha_percent: meta.z_blend_minimum_alpha_percent,
            z_blend_max_alpha_percent: meta.z_blend_max_alpha_percent,
            z_blend_custom_lut: meta.z_blend_custom_lut,
            zaa_kernel: meta.zaa_kernel,
            zaa_pattern: meta.zaa_pattern,
            zaa_duplicate_z: meta.zaa_duplicate_z,
            dither_enabled: meta.dither_enabled,
            dither_bit_depth: meta.dither_bit_depth,
            dither_device_gamma: meta.dither_device_gamma,
            container_compression_level: meta.container_compression_level,
            build_width_mm: meta.build_width_mm,
            build_depth_mm: meta.build_depth_mm,
            layer_height_mm: meta.layer_height_mm,
            total_layers: meta.total_layers,
            export_thumbnail_png_base64: meta.export_thumbnail_png_base64,
            triangles_xyz,
            metadata_json: meta.metadata_json,
        };

        log::info!(
            "[SupportAA] native job decoded: model_triangles={} support_triangles={} total_triangles={} aa_on_supports={} mode={} level={} mesh_encoding={}",
            job.model_triangle_count,
            (job.triangles_xyz.len() / 9).saturating_sub(job.model_triangle_count as usize),
            job.triangles_xyz.len() / 9,
            job.aa_on_supports,
            job.anti_aliasing_mode,
            job.anti_aliasing_level,
            meta.mesh_encoding.as_deref().unwrap_or("raw_f32"),
        );

        let progress_cb = make_throttled_progress_cb(sink);

        let (path_text, byte_len, perf, runtime) = slicer_pool().install(
            || -> BackendResult<(String, u64, NativeSlicerPerfMetrics, NativeSlicerRuntimeMetrics)> {
                let ext = if job.output_format.trim().is_empty() {
                    DEFAULT_EXPORT_FORMAT.to_string()
                } else {
                    job.output_format.trim_start_matches('.').to_string()
                };
                let path = if let Some(requested_path) = requested_output_path.as_deref() {
                    let requested = PathBuf::from(requested_path);
                    if let Some(parent) = requested.parent() {
                        std::fs::create_dir_all(parent)
                            .map_err(|err| format!("Failed creating output folder: {err}"))?;
                    }
                    requested
                } else {
                    crate::files::temp_artifact_path(&ext)
                };

                let perf_raw = dragonfruit_slicing_engine::engine::slice_with_progress_v3_to_path(
                    &job,
                    &path,
                    progress_cb,
                    Some(flag.as_ref()),
                )
                .map_err(|err| format!("V3 slicing failed: {err}"))?;

                let perf = NativeSlicerPerfMetrics {
                    total_ns: perf_raw.total_ns,
                    index_build_ns: perf_raw.index_build_ns,
                    render_wall_ns: perf_raw.render_wall_ns,
                    render_ns: perf_raw.render_ns,
                    png_encode_ns: perf_raw.png_encode_ns,
                    archive_encode_ns: perf_raw.archive_encode_ns,
                    z_blend_backward_ns: perf_raw.z_blend_backward_ns,
                    z_blend_forward_ns: perf_raw.z_blend_forward_ns,
                    cross_blend_ns: perf_raw.cross_blend_ns,
                    cross_blend_touched_pixels: perf_raw.cross_blend_touched_pixels,
                    cross_blend_contributing_layers: perf_raw.cross_blend_contributing_layers,
                    post_blur_ns: perf_raw.post_blur_ns,
                    support_merge_ns: perf_raw.support_merge_ns,
                    layers: perf_raw.layers,
                };

                let artifact_metadata_start = std::time::Instant::now();
                let byte_len = std::fs::metadata(&path)
                    .map_err(|err| format!("Failed reading temp artifact metadata: {err}"))?
                    .len();
                let artifact_metadata_ns = duration_ns_u64(artifact_metadata_start.elapsed());

                let wrapper_total_ns = duration_ns_u64(wrapper_start.elapsed());
                let wrapper_overhead_ns = wrapper_total_ns.saturating_sub(perf_raw.total_ns);
                let runtime = v3_runtime_metrics(
                    &path,
                    perf_raw.daa_post_threads,
                    perf_raw.daa_post_buffer_depth,
                    metadata_parse_ns,
                    mesh_decode_ns,
                    artifact_metadata_ns,
                    wrapper_total_ns,
                    wrapper_overhead_ns,
                );

                Ok((path.to_string_lossy().to_string(), byte_len, perf, runtime))
            },
        )?;

        Ok(NativeSliceTempPathResult {
            temp_path: path_text,
            byte_len,
            perf: Some(perf),
            runtime,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_decode_requires_a_whole_number_of_floats() {
        let err = bytes_to_f32_soup(&[0u8; 5]).expect_err("refuse");
        assert!(err.contains("not a multiple of 4"), "{err}");
    }

    #[test]
    fn both_decoders_agree_on_a_soup() {
        let values = [1.0f32, -2.5, 0.25];
        let mut bytes = Vec::new();
        for value in values {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        assert_eq!(bytes_to_f32_soup(&bytes).expect("borrowed"), values);
        assert_eq!(bytes_into_f32_soup(bytes).expect("owned"), values);
    }

    #[test]
    fn quantized_mesh_maps_through_the_bounding_box() {
        let quant = MeshQuantizationMetadata {
            min_x: 0.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 10.0,
            max_y: 10.0,
            max_z: 10.0,
        };
        // One vertex at the low corner, one at the high corner.
        let mut bytes = Vec::new();
        for value in [0u16, 0, 0, 65_535, 65_535, 65_535] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        let decoded = decode_quantized_u16_mesh(&bytes, &quant).expect("decode");
        assert_eq!(decoded[0], 0.0);
        assert_eq!(decoded[4], 10.0);
    }

    #[test]
    fn a_degenerate_axis_decodes_to_its_minimum() {
        let quant = MeshQuantizationMetadata {
            min_x: 5.0,
            min_y: 0.0,
            min_z: 0.0,
            max_x: 5.0,
            max_y: 1.0,
            max_z: 1.0,
        };
        let bytes = 1u16.to_le_bytes();
        assert_eq!(decode_quantized_u16_mesh(&bytes, &quant).expect("decode"), vec![5.0]);
    }

    #[test]
    fn the_throttle_always_delivers_phase_changes_and_the_final_event() {
        use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};

        let seen = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&seen);
        let sink: SliceSink = Arc::new(move |_payload: SliceProgressPayload| {
            counter.fetch_add(1, AtomicOrdering::SeqCst);
        });
        let cb = make_throttled_progress_cb(Some(sink)).expect("callback");

        // Two Slicing updates in the same millisecond: the first opens the phase,
        // the second is inside the window and is dropped.
        cb(SliceProgressUpdateV3 {
            done: 1,
            total: 10,
            phase: SliceProgressPhaseV3::Slicing,
        });
        cb(SliceProgressUpdateV3 {
            done: 2,
            total: 10,
            phase: SliceProgressPhaseV3::Slicing,
        });
        // A phase change always goes.
        cb(SliceProgressUpdateV3 {
            done: 5,
            total: 10,
            phase: SliceProgressPhaseV3::Encoding,
        });
        // So does the last update.
        cb(SliceProgressUpdateV3 {
            done: 10,
            total: 10,
            phase: SliceProgressPhaseV3::Finalizing,
        });
        // Three guaranteed deliveries (first, phase change, final). The second
        // same-phase update is dropped *unless* this machine stalled for 8 ms
        // between two calls, so the window is the assertion.
        assert!(
            (3..=4).contains(&seen.load(AtomicOrdering::SeqCst)),
            "delivered {} of 4 updates",
            seen.load(AtomicOrdering::SeqCst)
        );
    }

    #[test]
    fn no_sink_means_no_callback() {
        assert!(make_throttled_progress_cb(None).is_none());
    }

    #[test]
    fn cancelling_flips_the_shared_flag_and_a_new_slice_clears_it() {
        let slicer = Slicer::new();
        assert!(!slicer.is_cancelled());
        slicer.cancel_slicing();
        assert!(slicer.is_cancelled());
    }

    #[test]
    fn a_job_without_a_staged_mesh_is_refused() {
        let staging = MeshStaging::new();
        let slicer = Slicer::new();
        let err = slicer
            .slice_solid_native_to_temp_path(&staging, "{}", None)
            .expect_err("no mesh");
        assert!(err.contains("No staged mesh"), "{err}");
    }
}
