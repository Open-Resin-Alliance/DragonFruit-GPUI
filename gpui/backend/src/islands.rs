//! `run_island_scan_native` — the streaming island scan behind Volume Analysis.
//!
//! Ported from `src-tauri/src/main.rs` lines 1859-2210. The scan itself is
//! `dragonfruit-islands::stream::run_island_scan_streaming`; the shell module was
//! the grid derivation, the run, and the reshaping of the crate's result into the
//! `ScanResults` shape the overlay renders (including the derived `firstHit` /
//! `lastHit` / `baseLabels` / `compBase` / `compTop` arrays, which the client-side
//! detector used to compute).
//!
//! ## Why the other island commands are not here
//!
//! `scan_mesh_minima`, `scan_mesh_minima_from_path`, `scan_voxel_islands_from_path`,
//! `scan_islands_from_path` and `scan_islands_staged` are **deferred**: their
//! algorithms (the local-minima scan with its sonar context, the surface snap, the
//! voxel island tracking and the combined scan) live in
//! `src-tauri/src/mesh_minima.rs` rather than in a `rust/` crate, so there is
//! nothing here to delegate to. See `docs/internal/gpui-backend-map.md`.

use dragonfruit_islands::model::{
    Connectivity, GridRef, IslandScanJob, IslandStatus,
};
use dragonfruit_islands::stream::run_island_scan_streaming;
use serde::{Deserialize, Serialize};

use crate::staging::MeshStaging;
use crate::BackendResult;

fn default_connectivity() -> u8 {
    4
}
fn default_min_island_area() -> f64 {
    0.01
}
fn default_overlap_px() -> i32 {
    1
}
fn default_neighborhood_px() -> i32 {
    1
}

/// Parameters for the native island scan. Field names match the JSON the frontend
/// sends (snake_case), not the camelCase the results use.
#[derive(Deserialize)]
struct IslandScanParams {
    px_mm: f64,
    support_buffer_mm: f64,
    #[serde(default = "default_connectivity")]
    connectivity: u8,
    #[serde(default = "default_min_island_area")]
    min_island_area_mm2: f64,
    #[serde(default = "default_overlap_px")]
    min_overlap_px: i32,
    #[serde(default = "default_neighborhood_px")]
    overlap_neighborhood_px: i32,
    layer_height_mm: f64,
    // Bounding box from the frontend, in world coordinates after the transform.
    bbox_min_x: f64,
    bbox_max_x: f64,
    bbox_min_y: f64,
    bbox_max_y: f64,
    bbox_min_z: f64,
    bbox_max_z: f64,
}

/// The scan's result, shaped to match the renderer's `ScanResults`.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeIslandScanResult {
    pub grid: NativeGridRef,
    pub islands: Vec<NativeIsland>,
    pub island_labels_per_layer: Vec<NativeRleLabels>,
    // Derived arrays the overlay needs.
    pub first_hit: Vec<i16>,
    pub last_hit: Vec<i16>,
    pub base_footprint: Vec<u8>,
    pub base_labels: Vec<i32>,
    pub comp_base: Vec<i16>,
    pub comp_top: Vec<i16>,
    // Perf, in milliseconds.
    pub rasterize_ms: f64,
    pub scan_ms: f64,
    pub total_ms: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeGridRef {
    pub origin_x: f64,
    pub origin_z: f64,
    pub width: i32,
    pub height: i32,
    pub px_mm: f64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeIsland {
    pub id: u32,
    pub first_layer: u32,
    pub last_layer: u32,
    pub status: String,
    pub total_area_mm2: f64,
    pub per_layer_area_mm2: std::collections::HashMap<String, f64>,
    pub parent_id: Option<u32>,
    pub child_ids: Vec<u32>,
    pub volume_mm3: Option<f64>,
    pub max_area_mm2: Option<f64>,
    pub max_area_layer: Option<u32>,
    pub is_merged_placeholder: bool,
    pub centroid: Option<NativeCentroid>,
    pub last_layer_centroid: Option<NativeCentroid>,
}

#[derive(Serialize)]
pub struct NativeCentroid {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

/// One layer's label runs, flattened to `[start, length, id, …]` per row.
#[derive(Serialize)]
pub struct NativeRleLabels {
    pub rows: Vec<Vec<i32>>,
    pub width: i32,
    pub height: i32,
}

/// `run_island_scan_native`: scan the staged soup for islands.
///
/// The staged mesh must have been uploaded in memory (`stage_mesh_binary_*`); the
/// shell refused the file variant here, and so does this.
///
/// Progress event: `islandscan://progress`, with phase `"Scanning"`.
pub fn run_island_scan(
    staging: &MeshStaging,
    params_json: &str,
    on_progress: Option<&(dyn Fn(u32, u32) + Sync)>,
) -> BackendResult<NativeIslandScanResult> {
    let mesh_bytes = staging
        .take_memory_buffer()?
        .ok_or("No staged mesh binary — call stage_mesh_binary first")?;

    let params: IslandScanParams = serde_json::from_str(params_json)
        .map_err(|e| format!("Invalid island scan params JSON: {e}"))?;

    let triangles_xyz = crate::slicing::bytes_to_f32_soup(&mesh_bytes)?;
    let triangles = dragonfruit_slicing_engine::geometry::parse_triangles(&triangles_xyz);

    log::debug!(
        "[island-scan-native] triangles={} bbox=({:.4},{:.4},{:.4})-({:.4},{:.4},{:.4}) px_mm={} layer_h={} buf={} conn={} min_area={} overlap_px={} neighborhood={}",
        triangles.len(),
        params.bbox_min_x, params.bbox_min_y, params.bbox_min_z,
        params.bbox_max_x, params.bbox_max_y, params.bbox_max_z,
        params.px_mm, params.layer_height_mm, params.support_buffer_mm,
        params.connectivity, params.min_island_area_mm2,
        params.min_overlap_px, params.overlap_neighborhood_px,
    );

    // Grid dimensions and bounds.
    let origin_x = params.bbox_min_x;
    // The mask's Y axis is the negated world Y: the container's pixel grid grows
    // downward, the scene's up.
    let origin_z = -params.bbox_max_y;
    let grid_width = ((params.bbox_max_x - params.bbox_min_x) / params.px_mm)
        .ceil()
        .max(1.0) as i32;
    let grid_height = ((params.bbox_max_y - params.bbox_min_y) / params.px_mm)
        .ceil()
        .max(1.0) as i32;
    let model_height = params.bbox_max_z - params.bbox_min_z;
    let num_layers = (model_height / params.layer_height_mm).ceil().max(0.0) as usize;

    let w = grid_width as usize;
    let h = grid_height as usize;

    let t_scan = std::time::Instant::now();
    let connectivity = if params.connectivity == 8 {
        Connectivity::Eight
    } else {
        Connectivity::Four
    };

    let job = IslandScanJob {
        px_mm: params.px_mm,
        support_buffer_mm: params.support_buffer_mm,
        connectivity,
        min_island_area_mm2: params.min_island_area_mm2,
        layer_height_mm: params.layer_height_mm,
        grid: GridRef {
            origin_x,
            origin_z,
            width: grid_width,
            height: grid_height,
            px_mm: params.px_mm,
        },
        num_layers: num_layers as u32,
        min_overlap_px: params.min_overlap_px,
        overlap_neighborhood_px: params.overlap_neighborhood_px,
        candidate_only: false,
    };

    // `store_labels = true`: the volume analysis reads the per-layer labels back.
    let scan_result = crate::slicing::slicer_pool().install(|| {
        run_island_scan_streaming(
            &job,
            &triangles,
            params.bbox_min_z,
            params.bbox_max_z,
            true,
            on_progress,
        )
    });
    let scan_ms = t_scan.elapsed().as_secs_f64() * 1000.0;
    // The streaming scanner rasterises as it goes, so there is no separate
    // rasterise phase to time.
    let rasterize_ms = 0.0;
    let total_ms = scan_ms;

    let total_solid_px: u64 = scan_result
        .island_labels_per_layer
        .iter()
        .map(|labels| {
            labels
                .rows
                .iter()
                .map(|row| row.iter().map(|run| run.length as u64).sum::<u64>())
                .sum::<u64>()
        })
        .sum();

    log::info!(
        "[island-scan-native] grid={}x{} layers={} solid_px={} islands={} scan={:.0}ms",
        grid_width,
        grid_height,
        num_layers,
        total_solid_px,
        scan_result.islands.len(),
        scan_ms,
    );

    // Derived per-pixel arrays.
    let pixel_count = w * h;
    let mut first_hit = vec![-1i16; pixel_count];
    let mut last_hit = vec![-1i16; pixel_count];
    let mut base_labels = vec![0i32; pixel_count];

    for (l, labels) in scan_result.island_labels_per_layer.iter().enumerate() {
        for (y, row) in labels.rows.iter().enumerate() {
            let row_off = y * w;
            for run in row {
                for j in 0..run.length {
                    let idx = row_off + (run.start + j) as usize;
                    if idx < pixel_count {
                        if first_hit[idx] == -1 {
                            first_hit[idx] = l as i16;
                            if run.id > 0 {
                                base_labels[idx] = run.id;
                            }
                        }
                        last_hit[idx] = l as i16;
                    }
                }
            }
        }
    }

    let base_footprint: Vec<u8> = first_hit
        .iter()
        .map(|&h| if h != -1 { 1 } else { 0 })
        .collect();

    let max_id = scan_result.islands.iter().map(|i| i.id.0).max().unwrap_or(0) as usize;
    let mut comp_base = vec![-1i16; max_id + 1];
    let mut comp_top = vec![-1i16; max_id + 1];
    for island in &scan_result.islands {
        let id = island.id.0 as usize;
        if id <= max_id {
            comp_base[id] = island.first_layer as i16;
            comp_top[id] = island.last_layer as i16;
        }
    }

    let islands: Vec<NativeIsland> = scan_result
        .islands
        .iter()
        .map(|i| NativeIsland {
            id: i.id.0,
            first_layer: i.first_layer,
            last_layer: i.last_layer,
            status: match i.status {
                IslandStatus::Active => "active".into(),
                IslandStatus::Complete => "complete".into(),
            },
            total_area_mm2: i.total_area_mm2,
            per_layer_area_mm2: i
                .per_layer_area_mm2
                .iter()
                .map(|(k, v)| (k.to_string(), *v))
                .collect(),
            parent_id: i.parent_id.map(|p| p.0),
            child_ids: i.child_ids.iter().map(|c| c.0).collect(),
            volume_mm3: i.volume_mm3,
            max_area_mm2: i.max_area_mm2,
            max_area_layer: i.max_area_layer,
            is_merged_placeholder: i.is_merged_placeholder,
            centroid: i.centroid.map(|c| NativeCentroid {
                x: c.x,
                y: c.y,
                z: c.z,
            }),
            last_layer_centroid: i.last_layer_centroid.map(|c| NativeCentroid {
                x: c.x,
                y: c.y,
                z: c.z,
            }),
        })
        .collect();

    let island_labels_per_layer: Vec<NativeRleLabels> = scan_result
        .island_labels_per_layer
        .iter()
        .map(|labels| NativeRleLabels {
            rows: labels
                .rows
                .iter()
                .map(|row| {
                    let mut flat = Vec::with_capacity(row.len() * 3);
                    for run in row {
                        flat.push(run.start);
                        flat.push(run.length);
                        flat.push(run.id);
                    }
                    flat
                })
                .collect(),
            width: labels.width,
            height: labels.height,
        })
        .collect();

    Ok(NativeIslandScanResult {
        grid: NativeGridRef {
            origin_x,
            origin_z,
            width: grid_width,
            height: grid_height,
            px_mm: params.px_mm,
        },
        islands,
        island_labels_per_layer,
        first_hit,
        last_hit,
        base_footprint,
        base_labels,
        comp_base,
        comp_top,
        rasterize_ms,
        scan_ms,
        total_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 10 mm cube, wound outward, as the staged soup format carries it.
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

    fn params_json() -> String {
        serde_json::json!({
            "px_mm": 0.05,
            "support_buffer_mm": 0.2,
            "layer_height_mm": 0.05,
            "bbox_min_x": 0.0, "bbox_max_x": 10.0,
            "bbox_min_y": 0.0, "bbox_max_y": 10.0,
            "bbox_min_z": 0.0, "bbox_max_z": 10.0,
        })
        .to_string()
    }

    #[test]
    fn a_solid_cube_scans_as_one_island() {
        let soup = cube_soup();
        let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();
        let staging = MeshStaging::new();
        staging.stage_mesh_binary_set(&bytes).expect("stage");

        let result = run_island_scan(&staging, &params_json(), None).expect("scan");
        assert_eq!(result.grid.width, 200);
        assert_eq!(result.grid.height, 200);
        // The mask's Z origin is the negated world Y maximum.
        assert_eq!(result.grid.origin_z, -10.0);
        // The cube rasterises (the first layer is a full square), and the derived
        // arrays are sized to the grid and to the island table.
        assert!(
            result.first_hit.iter().any(|&hit| hit != -1),
            "the cube must occupy at least one pixel"
        );
        assert_eq!(result.base_footprint.len(), 200 * 200);
        assert_eq!(result.comp_base.len(), result.comp_top.len());
        let max_id = result.islands.iter().map(|i| i.id).max().unwrap_or(0) as usize;
        assert_eq!(result.comp_base.len(), max_id + 1);
        // The staged buffer is consumed, as the shell's scan consumed it.
        assert!(staging.take_memory_buffer().expect("take").is_none());
    }

    #[test]
    fn a_scan_without_a_staged_mesh_is_refused() {
        let staging = MeshStaging::new();
        let err = run_island_scan(&staging, &params_json(), None).expect_err("no mesh");
        assert!(err.contains("stage_mesh_binary"), "{err}");
    }

    #[test]
    fn malformed_params_are_refused_before_the_mesh_is_consumed() {
        let staging = MeshStaging::new();
        let soup = cube_soup();
        let bytes: Vec<u8> = bytemuck::cast_slice::<f32, u8>(&soup).to_vec();
        staging.stage_mesh_binary_set(&bytes).expect("stage");
        // The mesh is taken first, then the params fail — the shell's order.
        let err = run_island_scan(&staging, "{not json", None).expect_err("bad params");
        assert!(err.contains("Invalid island scan params JSON"), "{err}");
    }
}
