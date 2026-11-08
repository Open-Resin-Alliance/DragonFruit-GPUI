//! The density grid: boundary ring + lattice infill for one overhang region.
//!
//! A port of `src/supports/autoSupport/gridPlacement.ts` — `computeRegionSpacing`,
//! `shouldUseDensityGrid`, `steepFlatSpacingMultiplier`, `steepFlatAnchorBandTop`,
//! `erodeFootprint`, `buildBoundaryPoints`, `sampleBoundary2D`,
//! `createTriangleSurfaceAt`, `createVoxelSurfaceAt`, `faceNormalAt` and
//! `generateGridCandidates`.
//!
//! # PORT DEVIATIONS
//!
//! - **The surface sampler walks the region's own triangles.** The TS installs
//!   a whole-mesh raycaster and then keeps only hits whose `faceIndex` is in
//!   `island.triangleIds`; intersecting just those triangles answers the same
//!   question without a BVH.
//! - **`applyContactTipCaps` is not run.** It shrinks `tipDiameterMm`, which is
//!   contact *sizing*; the contact positions and normals are unaffected.
//! - **`isSupportBlockedContact` is not run.** It refuses contacts inside a
//!   user-painted no-go mask, which a pure function has no state for.

use std::collections::{HashMap, HashSet};

use glam::{DVec3, Vec3};

use super::candidates::{Candidate, CandidateSource, Island};
use super::constants::{
    FOOTPRINT_TOLERANCE_MM, GRID_SPACING_FLOOR_MM, GRID_SPACING_MAX_FACTOR,
    GRID_SPACING_MIN_FACTOR, INTERIOR_CAVITY_SPACING_SCALE, ISLAND_TWO_POINT_MAX_MM,
    MAX_GRID_CANDIDATES_PER_REGION, OVERHANG_SELF_SUPPORT_ANGLE_DEG, PERIMETER_CONTACT_INSET_MM,
    PERIMETER_RING_FACTOR, PERIMETER_SPACING_FLOOR_MM, SLOPE_RELAX_RAMP_START,
    STEEP_FLAT_ANCHOR_BAND_FRACTION, STEEP_FLAT_ANCHOR_BAND_MM, STEEP_FLAT_SPACING_MULTIPLIER,
};
use super::settings::SupportSettings;
use super::scan::{barycentric_z, point_in_triangle_2d};
use super::soup::WeldedMesh;

/// The integer key of a 0.25 mm footprint cell (`cellKey`).
fn cell_key(cx: i64, cy: i64) -> i64 {
    (cx + 32768) * 65536 + (cy + 32768)
}

/// JS `Math.round`: half-way values round toward +∞.
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// A surface sample: the face's Z plus the mesh face it came from.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    z: f32,
    /// Welded-mesh face index, or `None` for the voxel fallback.
    face: Option<u32>,
}

/// `SurfaceSampler` — `{ z, faceIndex }` or null when the point lies outside
/// the region.
pub(crate) enum SurfaceAt<'a> {
    /// `createTriangleSurfaceAt`: an upward ray from just below the region's
    /// base, accepted only on one of the region's own triangles.
    Triangles {
        mesh: &'a WeldedMesh,
        ids: &'a [u32],
        base_z: f32,
    },
    /// `createVoxelSurfaceAt`: the nearest footprint voxel within
    /// [`FOOTPRINT_TOLERANCE_MM`] supplies containment and Z.
    Voxels {
        points: Vec<[f32; 3]>,
        cell_size: f32,
        min_z: f32,
        buckets: HashMap<(i32, i32), Vec<usize>>,
    },
}

impl<'a> SurfaceAt<'a> {
    fn triangles(mesh: &'a WeldedMesh, ids: &'a [u32], base_z: f32) -> Self {
        Self::Triangles { mesh, ids, base_z }
    }

    /// `createVoxelSurfaceAt(points, cellSize, minZ)`.
    fn voxels(points: Vec<[f32; 3]>, cell_size: f32, min_z: f32) -> Self {
        let mut buckets: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
        for (index, p) in points.iter().enumerate() {
            buckets
                .entry((
                    (p[0] / cell_size).floor() as i32,
                    (p[1] / cell_size).floor() as i32,
                ))
                .or_default()
                .push(index);
        }
        Self::Voxels {
            points,
            cell_size,
            min_z,
            buckets,
        }
    }

    fn sample(&self, x: f64, y: f64) -> Option<Sample> {
        match self {
            Self::Triangles { mesh, ids, base_z } => {
                // `createTriangleSurfaceAt` seeds at `island.baseZ - 2`.
                let origin_z = *base_z - 2.0;
                let x = x as f32;
                let y = y as f32;
                let mut best: Option<Sample> = None;
                for &face in ids.iter() {
                    let [a, b, c] = mesh.tri_positions(face);
                    if !point_in_triangle_2d(x, y, (a.x, a.y), (b.x, b.y), (c.x, c.y)) {
                        continue;
                    }
                    let z = barycentric_z(x, y, a, b, c);
                    // The ray runs +Z from `base_z - 2`; a hit below that is
                    // not on this ray.
                    if z < origin_z {
                        continue;
                    }
                    if best.is_none_or(|current| z < current.z) {
                        best = Some(Sample { z, face: Some(face) });
                    }
                }
                best
            }
            Self::Voxels {
                points,
                cell_size,
                min_z,
                buckets,
            } => {
                let tol_sq = (FOOTPRINT_TOLERANCE_MM * FOOTPRINT_TOLERANCE_MM) as f32;
                let gx = (x as f32 / cell_size).floor() as i32;
                let gy = (y as f32 / cell_size).floor() as i32;
                let mut best_d2 = f32::INFINITY;
                let mut best_z = *min_z;
                for dx in -1..=1 {
                    for dy in -1..=1 {
                        let Some(bucket) = buckets.get(&(gx + dx, gy + dy)) else {
                            continue;
                        };
                        for &index in bucket {
                            let p = points[index];
                            let ddx = x as f32 - p[0];
                            let ddy = y as f32 - p[1];
                            let d2 = ddx * ddx + ddy * ddy;
                            if d2 <= tol_sq && d2 < best_d2 {
                                best_d2 = d2;
                                best_z = p[2];
                            }
                        }
                    }
                }
                if best_d2 <= tol_sq {
                    Some(Sample {
                        z: best_z,
                        face: None,
                    })
                } else {
                    None
                }
            }
        }
    }
}

/// `faceNormalAt`: the world normal of one mesh face, normalized but not
/// re-oriented.
fn face_normal_at(mesh: &WeldedMesh, face: u32) -> Option<Vec3> {
    let [a, b, c] = mesh.tri_positions(face);
    let normal = (b - a).cross(c - a);
    if normal.length_squared() < 1e-12 {
        return None;
    }
    Some(normal.normalize())
}

/// `computeRegionSpacing`: the shared density formula.
///
/// ```text
/// spacing = max(floor, √areaPerSupport × (flat + angleT × (relax − flat)) × anchor)
/// flat    = flatDensityBoost × (threshold / area)^suctionAreaExponent   (area > threshold)
/// ```
pub(crate) fn compute_region_spacing(island: &Island, settings: &SupportSettings) -> f64 {
    let base_spacing = ((settings.area_per_support_mm2.max(0.5)) as f64).sqrt();
    let self_support_angle_deg = if settings.overhang_self_support_angle_deg.is_finite() {
        settings.overhang_self_support_angle_deg as f64
    } else {
        OVERHANG_SELF_SUPPORT_ANGLE_DEG as f64
    };
    // Relaxation is a last-stretch effect, not a ramp from flat.
    let relax_start_deg = self_support_angle_deg * SLOPE_RELAX_RAMP_START;
    let ramp_deg = (self_support_angle_deg - relax_start_deg).max(1e-6);
    let angle_t = (((island.overhang_angle_deg as f64) - relax_start_deg) / ramp_deg).clamp(0.0, 1.0);
    let min_factor = if settings.flat_density_boost.is_finite() {
        settings.flat_density_boost as f64
    } else {
        GRID_SPACING_MIN_FACTOR
    };
    let max_factor = if settings.slope_relax_factor.is_finite() {
        settings.slope_relax_factor as f64
    } else {
        GRID_SPACING_MAX_FACTOR
    };

    let area = island.area_mm2 as f64;
    let threshold = settings.grid_area_threshold_mm2 as f64;
    let exponent = settings.suction_area_exponent as f64;
    let area_scale = if exponent > 0.0 && area > threshold {
        (threshold / area).powf(exponent)
    } else {
        1.0
    };

    let flat_factor = min_factor * area_scale;
    let spacing = base_spacing * (flat_factor + angle_t * (max_factor - flat_factor));
    spacing.max(GRID_SPACING_FLOOR_MM)
}

/// `steepFlatSpacingMultiplier`: the sparse field a steep flat gets, capped by
/// the part's own thickness because the sag between two contacts goes as the
/// span to the fourth power.
pub(crate) fn steep_flat_spacing_multiplier(
    island: &Island,
    base_spacing_mm: f64,
    part_thickness_mm: f64,
) -> f64 {
    if !island.steep_flat {
        return 1.0;
    }
    let sparse = STEEP_FLAT_SPACING_MULTIPLIER;
    if !(part_thickness_mm > 0.0) || !(base_spacing_mm > 0.0) {
        return sparse;
    }
    (part_thickness_mm / base_spacing_mm).clamp(1.0, sparse)
}

/// `steepFlatAnchorBandTop`: top of the band a steep flat's anchoring contacts
/// may occupy (mm), or `f64::INFINITY` for anything that is not a steep flat.
pub(crate) fn steep_flat_anchor_band_top(island: &Island, slender_part: bool) -> f64 {
    if !island.steep_flat {
        return f64::INFINITY;
    }
    if slender_part {
        return f64::INFINITY;
    }
    let base_z = island.base_z as f64;
    let top = island.max_z as f64;
    base_z + ((top - base_z) * STEEP_FLAT_ANCHOR_BAND_FRACTION).max(STEEP_FLAT_ANCHOR_BAND_MM)
}

/// `shouldUseDensityGrid`: area decides the density question, shape decides
/// whether a sub-threshold region can be carried by one contact at all.
pub(crate) fn should_use_density_grid(island: &Island, settings: &SupportSettings) -> bool {
    if (island.area_mm2 as f64) >= settings.grid_area_threshold_mm2 as f64 {
        return true;
    }
    let Some(bounds) = footprint_bounds(&island.footprint.points()) else {
        return false;
    };
    // Longer than the two-point band: a thin rib needs its edge sampled,
    // whether or not it erodes away completely.
    let width = bounds.2 - bounds.0;
    let height = bounds.3 - bounds.1;
    width.max(height) >= ISLAND_TWO_POINT_MAX_MM
}

/// World-space XY bounds of a footprint.
fn footprint_bounds(points: &[[f32; 3]]) -> Option<(f32, f32, f32, f32)> {
    if points.is_empty() {
        return None;
    }
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for [x, y, _] in points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    Some((min_x, min_y, max_x, max_y))
}

/// `erodeFootprint`: mask pixels whose 2-pixel neighbourhood is fully present.
/// The region shrunken so a support centred on its boundary keeps its whole
/// contact disc on the surface.
fn erode_footprint(points: &[[f32; 3]]) -> Vec<[f32; 3]> {
    let set: HashSet<i64> = points
        .iter()
        .map(|[x, y, _]| cell_key(js_round(*x as f64 * 4.0) as i64, js_round(*y as f64 * 4.0) as i64))
        .collect();
    let mut interior = Vec::new();
    for point in points {
        let kx = js_round(point[0] as f64 * 4.0) as i64;
        let ky = js_round(point[1] as f64 * 4.0) as i64;
        let mut all = true;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if !set.contains(&cell_key(kx + dx, ky + dy)) {
                    all = false;
                    break;
                }
            }
            if !all {
                break;
            }
        }
        if all {
            interior.push(*point);
        }
    }
    interior
}

/// `buildBoundaryPoints`: boundary voxels of a region's footprint, sorted by
/// angle around the centroid, sub-sampled at `spacing` intervals.
pub(crate) fn build_boundary_points(voxels: &[[f32; 3]], spacing: f64) -> Vec<[f32; 3]> {
    if voxels.is_empty() {
        return Vec::new();
    }
    let mut set: HashSet<i64> = HashSet::new();
    let mut sum_x = 0f64;
    let mut sum_y = 0f64;
    for point in voxels {
        set.insert(cell_key(
            js_round(point[0] as f64 * 4.0) as i64,
            js_round(point[1] as f64 * 4.0) as i64,
        ));
        sum_x += point[0] as f64;
        sum_y += point[1] as f64;
    }
    let count = voxels.len() as f64;
    let cx = sum_x / count;
    let cy = sum_y / count;

    let mut boundary: Vec<[f32; 3]> = Vec::new();
    for point in voxels {
        let kx = js_round(point[0] as f64 * 4.0) as i64;
        let ky = js_round(point[1] as f64 * 4.0) as i64;
        let mut on_edge = false;
        for dx in -1..=1 {
            for dy in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                if !set.contains(&cell_key(kx + dx, ky + dy)) {
                    on_edge = true;
                    break;
                }
            }
            if on_edge {
                break;
            }
        }
        if on_edge {
            boundary.push(*point);
        }
    }
    if boundary.is_empty() {
        return Vec::new();
    }

    boundary.sort_by(|a, b| {
        let angle_a = ((a[1] as f64) - cy).atan2((a[0] as f64) - cx);
        let angle_b = ((b[1] as f64) - cy).atan2((b[0] as f64) - cx);
        angle_a.partial_cmp(&angle_b).unwrap_or(std::cmp::Ordering::Equal)
    });
    // `Math.max(1, Math.round(spacing / 0.25))` — 0.25 mm is the footprint's own
    // cell pitch (`OVERHANG_FOOTPRINT_PX_MM`).
    let cell_pitch_mm = 0.25f64;
    let step = (js_round(spacing / cell_pitch_mm) as i64).max(1) as usize;
    let mut points = Vec::new();
    for index in (0..boundary.len()).step_by(step) {
        points.push(boundary[index]);
    }
    points
}

/// `resolveSurfaceNormal`: resolve the real surface normal at a candidate's tip
/// by casting along Z, walking a small disc of offsets before giving up.
///
/// Both rays walk radii `[0, 0.75, 1.5, 2.25]` (eight steps each, one at radius
/// 0), and take the first hit still on the tip's side (`point.z >= tip.z - 0.5`).
/// The upward ray is the underside contact and its hit normal is the answer; the
/// downward fallback's normal is flipped so the support still grows away from
/// the face.
///
/// # PORT DEVIATIONS
///
/// - **The cast is limited to the region's own triangles.** `autoPlace.ts` casts
///   against the whole mesh and keeps the first hit, which for a contact sitting
///   on a region face is that face; walking the region's triangles answers the
///   same question without a BVH. `calculateSmoothedNormal` interpolates vertex
///   normals, which a triangle soup does not carry — the face normal is what
///   `MeshTriangles` can offer, and it is also what the grid path uses.
pub(crate) fn resolve_surface_normal(
    mesh: &WeldedMesh,
    ids: &[u32],
    tip: DVec3,
) -> (DVec3, DVec3) {
    /// `SEARCH_RADII_MM` in `resolveSurfaceNormal` (`autoPlace.ts`).
    const SEARCH_RADII_MM: [f64; 4] = [0.0, 0.75, 1.5, 2.25];
    /// `SAME_SIDE_TOLERANCE_MM` in `resolveSurfaceNormal`.
    const SAME_SIDE_TOLERANCE_MM: f64 = 0.5;
    /// The `±2` the ray is seeded at (`tipPos.z - 2` / `tipPos.z + 2`).
    const SEED_OFFSET_MM: f64 = 2.0;

    let cast = |direction_z: f64| -> Option<(DVec3, Vec3)> {
        let seed_z = if direction_z > 0.0 {
            tip.z - SEED_OFFSET_MM
        } else {
            tip.z + SEED_OFFSET_MM
        };
        for radius in SEARCH_RADII_MM {
            let steps = if radius == 0.0 { 1 } else { 8 };
            for step in 0..steps {
                let angle = step as f64 / steps as f64 * std::f64::consts::TAU;
                let ox = (tip.x + angle.cos() * radius) as f32;
                let oy = (tip.y + angle.sin() * radius) as f32;
                let mut best: Option<(f64, f32, u32)> = None;
                for &face in ids {
                    let [a, b, c] = mesh.tri_positions(face);
                    if !point_in_triangle_2d(ox, oy, (a.x, a.y), (b.x, b.y), (c.x, c.y)) {
                        continue;
                    }
                    let z = barycentric_z(ox, oy, a, b, c) as f64;
                    let distance = if direction_z > 0.0 {
                        z - seed_z
                    } else {
                        seed_z - z
                    };
                    if distance < 0.0 {
                        continue;
                    }
                    if z < tip.z - SAME_SIDE_TOLERANCE_MM {
                        continue;
                    }
                    if best.is_none_or(|(best_t, _, _)| distance < best_t) {
                        best = Some((distance, z as f32, face));
                    }
                }
                if let Some((_, z, face)) = best {
                    let normal = face_normal_at(mesh, face).unwrap_or(Vec3::NEG_Z);
                    return Some((DVec3::new(ox as f64, oy as f64, z as f64), normal));
                }
            }
        }
        None
    };

    // Primary: upward ray from just below the tip (underside contact).
    if let Some((point, normal)) = cast(1.0) {
        return (point, normal.as_dvec3());
    }
    // Fallback: downward ray from above (top-surface contact), normal flipped.
    if let Some((point, normal)) = cast(-1.0) {
        return (point, (-normal).as_dvec3());
    }
    // Keep the existing normal; the support still grows away from −Z.
    (tip, DVec3::NEG_Z)
}

/// `sampleBoundary2D`: resample the region's perimeter loops as a 2D-projected
/// boundary ring at fixed arc-length spacing. Returns `None` when the region
/// carries no usable loops.
pub(crate) fn sample_boundary_2d(
    loops: &[Vec<[f32; 3]>],
    spacing: f64,
    surface_at: &SurfaceAt<'_>,
) -> Option<Vec<[f32; 3]>> {
    if loops.is_empty() || spacing <= 0.0 {
        return None;
    }
    if !loops.iter().any(|loop_| loop_.len() >= 2) {
        return None;
    }

    let mut raw: Vec<[f32; 3]> = Vec::new();
    for loop_ in loops {
        let mut acc = 0f64;
        for i in 0..loop_.len() {
            let a = loop_[i];
            let b = loop_[(i + 1) % loop_.len()];
            let a0 = a[0] as f64;
            let a1 = a[1] as f64;
            let b0 = b[0] as f64;
            let b1 = b[1] as f64;
            let seg_len = (b0 - a0).hypot(b1 - a1);
            if seg_len < 1e-9 {
                continue;
            }
            if acc == 0.0 {
                if let Some(s) = surface_at.sample(a0, a1) {
                    raw.push([a[0], a[1], s.z]);
                }
            }
            let mut d = spacing - acc;
            while d <= seg_len {
                let t = d / seg_len;
                let x = a0 + (b0 - a0) * t;
                let y = a1 + (b1 - a1) * t;
                if let Some(s) = surface_at.sample(x, y) {
                    raw.push([x as f32, y as f32, s.z]);
                }
                d += spacing;
            }
            acc = (acc + seg_len) % spacing;
        }
    }

    let min_dist_sq = (spacing * 0.5) * (spacing * 0.5);
    let mut deduped: Vec<[f32; 3]> = Vec::new();
    for point in raw {
        let mut duplicate = false;
        for kept in &deduped {
            let dx = (point[0] - kept[0]) as f64;
            let dy = (point[1] - kept[1]) as f64;
            if dx * dx + dy * dy < min_dist_sq {
                duplicate = true;
                break;
            }
        }
        if !duplicate {
            deduped.push(point);
        }
    }
    if deduped.is_empty() {
        None
    } else {
        Some(deduped)
    }
}

/// `generateGridCandidates`: fixed-density boundary ring + grid infill for one
/// batch of overhang regions.
///
/// `slender_part` and `part_thickness_mm` are `poseStability`'s terms for a
/// steep flat; a run that has not measured the pose passes `false`/`0.0`, which
/// is also what `computeAutoSupportPlan` passes when no stability report is
/// available.
pub(crate) fn generate_grid_candidates(
    islands: &[&Island],
    settings: &SupportSettings,
    mesh: Option<&WeldedMesh>,
    slender_part: bool,
    part_thickness_mm: f64,
) -> Vec<Candidate> {
    let base_spacing = ((settings.area_per_support_mm2.max(0.5)) as f64).sqrt();
    if base_spacing <= 0.0 {
        return Vec::new();
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    for island in islands {
        if !should_use_density_grid(island, settings) {
            continue;
        }
        let region_spacing = compute_region_spacing(island, settings);
        // An internal ceiling is bridged to the cavity floor, not carried to
        // the plate; the short strut needs far fewer contacts.
        let spacing = region_spacing
            * steep_flat_spacing_multiplier(island, region_spacing, part_thickness_mm)
            * if island.interior {
                INTERIOR_CAVITY_SPACING_SCALE
            } else {
                1.0
            };

        let points = island.footprint.points();
        if points.is_empty() {
            continue;
        }
        let Some((min_x, min_y, max_x, max_y)) = footprint_bounds(&points) else {
            continue;
        };
        let cell_size = spacing.max(1.0) as f32;

        let surface_normal = Vec3::from_array(island.surface_normal);
        let surface_at = if let Some(mesh) = mesh {
            if !island.triangle_ids.is_empty() {
                SurfaceAt::triangles(mesh, &island.triangle_ids, island.base_z)
            } else {
                SurfaceAt::voxels(points.clone(), cell_size, island.base_z)
            }
        } else {
            SurfaceAt::voxels(points.clone(), cell_size, island.base_z)
        };
        // A steep flat is self-supporting, so its contacts ANCHOR the part.
        let anchor_band_top = steep_flat_anchor_band_top(island, slender_part);

        let emit_point = |candidates: &mut Vec<Candidate>,
                          x: f64,
                          y: f64,
                          z: f64,
                          kind: &str,
                          face: Option<u32>| {
            if z > anchor_band_top {
                return;
            }
            // The contact leans into the face it actually lands on; the
            // region-wide normal is only the fallback.
            let tip_normal = face
                .and_then(|face| mesh.and_then(|mesh| face_normal_at(mesh, face)))
                .unwrap_or(surface_normal);
            candidates.push(Candidate {
                id: format!("{kind}-{}-{:.2}-{:.2}", island.id, x, y),
                tip_pos: DVec3::new(x, y, z),
                tip_normal: tip_normal.as_dvec3(),
                source: CandidateSource::Overhang,
                island_area_mm2: settings.area_per_support_mm2 as f64,
                z_height: z,
                priority: 0.0,
                grid_point: true,
                region_index: island.region_index,
            });
        };

        // Sliver test: nothing survives footprint erosion → ring only.
        let eroded = erode_footprint(&points);
        let is_sliver = eroded.is_empty();

        // Candidate-cap guard: on overflow, fall back to the angle-only
        // spacing, then subsample evenly.
        let width = (max_x - min_x) as f64;
        let height = (max_y - min_y) as f64;
        let inset = PERIMETER_CONTACT_INSET_MM as f64;
        let span_x = (width - 2.0 * inset).max(0.5);
        let span_y = (height - 2.0 * inset).max(0.5);
        let mut stride: usize = 1;
        let estimate = ((span_x / spacing).floor() + 2.0) * ((span_y / spacing).floor() + 2.0);
        if estimate > MAX_GRID_CANDIDATES_PER_REGION as f64 {
            let mut no_suction = *settings;
            no_suction.suction_area_exponent = 0.0;
            let fallback_spacing = compute_region_spacing(island, &no_suction);
            let base_estimate = ((span_x / fallback_spacing).floor() + 2.0)
                * ((span_y / fallback_spacing).floor() + 2.0);
            if base_estimate > MAX_GRID_CANDIDATES_PER_REGION as f64 {
                stride = (base_estimate / MAX_GRID_CANDIDATES_PER_REGION as f64).ceil() as usize;
            }
        }

        let nx = (js_round(span_x / spacing) as i64).max(1);
        let ny = (js_round(span_y / spacing) as i64).max(1);
        let spacing_x = span_x / nx as f64;
        let spacing_y = span_y / ny as f64;

        // Grid infill — integer rows and columns spanning the region, inset by
        // the contact radius.
        let mut lattice: Vec<[f64; 3]> = Vec::new();
        if !is_sliver && width.min(height) >= spacing {
            let mut i = 0i64;
            while i <= nx {
                let mut j = 0i64;
                while j <= ny {
                    let x = min_x as f64 + inset + i as f64 * spacing_x;
                    let y = min_y as f64 + inset + j as f64 * spacing_y;
                    if let Some(s) = surface_at.sample(x, y) {
                        lattice.push([x, y, s.z as f64]);
                        emit_point(&mut candidates, x, y, s.z as f64, "grid", s.face);
                    }
                    j += stride as i64;
                }
                i += stride as i64;
            }
        }

        // Boundary ring — deliberately TIGHTER than the infill and deduped
        // against the lattice at half the ring spacing.
        let ring_spacing =
            (PERIMETER_SPACING_FLOOR_MM.max(spacing * PERIMETER_RING_FACTOR)) * stride as f64;
        let ring_coverage_sq = (ring_spacing * 0.5) * (ring_spacing * 0.5);
        let boundary = sample_boundary_2d(&island.perimeter_loops, ring_spacing, &surface_at)
            .unwrap_or_else(|| {
                build_boundary_points(if eroded.is_empty() { &points } else { &eroded }, ring_spacing)
            });
        for b in boundary {
            let mut covered = false;
            for p in &lattice {
                let dx = b[0] as f64 - p[0];
                let dy = b[1] as f64 - p[1];
                if dx * dx + dy * dy <= ring_coverage_sq {
                    covered = true;
                    break;
                }
            }
            if covered {
                continue;
            }
            // Fallback boundary points carry a voxel Z and no face: sample the
            // surface at their XY so the ring keeps the same per-cell Z and
            // normal the lattice got.
            let sampled = surface_at.sample(b[0] as f64, b[1] as f64);
            let z = sampled.map(|s| s.z).unwrap_or(b[2]);
            emit_point(
                &mut candidates,
                b[0] as f64,
                b[1] as f64,
                z as f64,
                "fill",
                sampled.and_then(|s| s.face),
            );
        }
    }

    candidates
}

#[cfg(test)]
mod tests {
    use super::*;

    fn island(area: f32, angle: f32) -> Island {
        Island {
            id: "o0".into(),
            region_index: 0,
            contact: Vec3::ZERO,
            base_z: 0.0,
            area_mm2: area,
            overhang_angle_deg: angle,
            surface_area_mm2: area,
            max_z: 0.0,
            steep_flat: false,
            triangle_ids: Vec::new(),
            surface_normal: [0.0, 0.0, -1.0],
            footprint: crate::generate::scan::FootprintMask::default(),
            perimeter_loops: Vec::new(),
            interior: false,
        }
    }

    #[test]
    fn flat_ceilings_grid_at_the_boosted_density() {
        let settings = SupportSettings::default();
        // √10 × 0.7, area <= threshold so the suction term is inert. The knobs
        // are f32, so the product carries their rounding.
        let spacing = compute_region_spacing(&island(20.0, 0.0), &settings);
        assert!((spacing - 10.0f64.sqrt() * 0.7).abs() < 1e-6, "{spacing}");
    }

    #[test]
    fn the_slope_relaxation_is_a_last_stretch() {
        let settings = SupportSettings::default();
        let at = |angle: f32| compute_region_spacing(&island(100.0, angle), &settings);
        let flat = at(0.0);
        assert_eq!(at(25.0), flat, "a 25° underside takes the flat spacing");
        assert_eq!(at(27.0), flat, "so does the start of the ramp (45° × 0.6)");
        assert!(at(44.0) > flat * 1.5, "44° must be well relaxed");
        assert!(at(44.0) < at(45.0));
    }

    #[test]
    fn spacing_never_falls_below_the_floor() {
        let mut settings = SupportSettings::default();
        settings.area_per_support_mm2 = 1.0;
        settings.flat_density_boost = 0.5;
        settings.slope_relax_factor = 1.0;
        settings.suction_area_exponent = 0.4;
        let spacing = compute_region_spacing(&island(400.0, 0.0), &settings);
        assert_eq!(spacing, GRID_SPACING_FLOOR_MM);
    }

    #[test]
    fn a_steep_flat_is_anchored_in_its_low_band() {
        let mut face = island(100.0, 60.0);
        face.steep_flat = true;
        face.base_z = 5.0;
        face.max_z = 55.0;
        assert_eq!(steep_flat_anchor_band_top(&face, false), 22.5);
        face.steep_flat = false;
        assert_eq!(steep_flat_anchor_band_top(&face, false), f64::INFINITY);
    }

    #[test]
    fn the_anchoring_spacing_follows_part_thickness() {
        let mut face = island(100.0, 60.0);
        face.steep_flat = true;
        let base = 3.0;
        assert_eq!(steep_flat_spacing_multiplier(&face, base, 7.0), 7.0 / base);
        assert_eq!(
            steep_flat_spacing_multiplier(&face, base, 30.0),
            STEEP_FLAT_SPACING_MULTIPLIER
        );
        assert_eq!(steep_flat_spacing_multiplier(&face, base, 2.0), 1.0);
        assert_eq!(
            steep_flat_spacing_multiplier(&face, base, 0.0),
            STEEP_FLAT_SPACING_MULTIPLIER
        );
    }

    /// A rectangular footprint mask at [`PERIMETER_CONTACT_INSET_MM`] spacing,
    /// built the way `build_footprint_mask` builds one.
    fn rect_mask(
        min_x: f32,
        max_x: f32,
        min_y: f32,
        max_y: f32,
        z: f32,
    ) -> crate::generate::scan::FootprintMask {
        let px = PERIMETER_CONTACT_INSET_MM;
        let width = ((max_x - min_x) / px).ceil() as u32;
        let height = ((max_y - min_y) / px).ceil() as u32;
        let origin_x = min_x - px * 0.5;
        let origin_y = min_y - px * 0.5;
        crate::generate::scan::FootprintMask {
            width,
            height,
            origin_x,
            origin_y,
            px_mm: px,
            data: vec![1; (width * height) as usize],
            surface_z: vec![z; (width * height) as usize],
        }
    }

    #[test]
    fn shape_decides_a_sub_threshold_rib() {
        let settings = SupportSettings::default();
        let mut rib = island(19.5, 0.0);
        rib.footprint = rect_mask(-7.5, 7.5, -0.65, 0.65, 10.0);
        assert!(should_use_density_grid(&rib, &settings));

        let mut patch = island(16.0, 0.0);
        patch.footprint = rect_mask(-2.0, 2.0, -2.0, 2.0, 10.0);
        assert!(!should_use_density_grid(&patch, &settings));
    }
}
