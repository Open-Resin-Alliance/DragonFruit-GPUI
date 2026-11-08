//! Islands → candidates → contacts: the filters, scoring and dedupe.
//!
//! A port of `overhangRegionToIsland` (`src/volumeAnalysis/Islands/useIslands.ts`),
//! `generateCandidates`/`candidateFromIsland`/`candidatesFromIsland`/
//! `computePriority`/`deduplicateCandidates` (`autoSupport/candidateGeneration.ts`)
//! and `influenceRadiusMm` (`autoSupport/constants.ts`).
//!
//! # PORT DEVIATIONS
//!
//! - **Overhang islands only.** The TS mixes voxel, minima and intersection
//!   islands from the slice-growth and minima scanners. Those need a layer
//!   stack or a vertex-minima scan; a mesh-only stage has neither, and its
//!   contacts are the overhang classifier's own regions. The minima branch of
//!   the area filter is therefore unreachable here.
//! - **Support-blocker pruning and tip caps are not applied.**
//!   `isSupportBlockedContact` reads a user-painted mask (no such state in a
//!   pure function) and `applyContactTipCaps` sets `tipDiameterMm`, a *sizing*
//!   field the placement stage later reads. Neither changes where a contact
//!   sits.
//! - **`priority` is computed for overhang candidates only**, as in the source:
//!   the `sourceScore` bonus belongs to `intersection`, which this stage never
//!   produces.

use glam::{DVec3, Vec3};

use super::constants::{
    INFLUENCE_RADIUS_KNOTS, ISLAND_SUB_HEAD_MM, ISLAND_TWO_POINT_MAX_MM,
    ISLAND_TWO_POINT_MAX_WIDTH_MM, ISLAND_TWO_POINT_MIN_MM, MAX_INFLUENCE_RADIUS_MM,
    STEEP_FLAT_ANCHOR_MIN_AREA_MM2, SUPPORT_RESTSTACK_DELTA_MM, TIP_COVERAGE_RADIUS_MM,
};
use super::scan::{FootprintMask, Region};
use super::settings::SupportSettings;

/// `DetectedIsland`, reduced to what contact placement reads.
#[derive(Clone, Debug)]
pub(crate) struct Island {
    /// `o<region index>` — `overhangRegionToIsland`'s id.
    pub id: String,
    /// Index of the source region, for tracing a contact back to the mesh.
    pub region_index: usize,
    /// Representative contact point: the footprint pixel with the lowest real
    /// surface Z.
    pub contact: Vec3,
    pub base_z: f32,
    /// `projectedAreaMm2` — the supportable footprint.
    pub area_mm2: f32,
    /// `overhangAngleDeg` — area-weighted mean angle from horizontal.
    pub overhang_angle_deg: f32,
    /// `surfaceAreaMm2` — the 3D area of the patch.
    pub surface_area_mm2: f32,
    pub max_z: f32,
    pub steep_flat: bool,
    pub triangle_ids: Vec<u32>,
    pub surface_normal: [f32; 3],
    pub footprint: FootprintMask,
    pub perimeter_loops: Vec<Vec<[f32; 3]>>,
    /// A tip on a cavity surface. Always false here: classifying interior
    /// contacts needs the cavity mesh (`createCavityTester`).
    pub interior: bool,
}

impl Island {
    /// `overhangRegionToIsland`: the contact is the footprint pixel with the
    /// lowest surface Z, so it lies ON the mesh even for sloped regions.
    pub fn from_region(index: usize, region: &Region) -> Self {
        let mask = &region.footprint;
        let mut best: Option<(f32, f32, f32)> = None;
        for row in 0..mask.height {
            for col in 0..mask.width {
                let idx = (row * mask.width + col) as usize;
                if mask.data[idx] == 0 {
                    continue;
                }
                let z = mask.surface_z[idx];
                if !z.is_finite() {
                    continue;
                }
                if best.is_none_or(|(_, _, best_z)| z < best_z) {
                    best = Some((
                        mask.origin_x + (col as f32 + 0.5) * mask.px_mm,
                        mask.origin_y + (row as f32 + 0.5) * mask.px_mm,
                        z,
                    ));
                }
            }
        }
        // Lowest sampled surface pixel; falls back to the mask centre at
        // `region.min_z` if the mask or its surface samples are unusable.
        let (contact_x, contact_y, contact_z) = best.unwrap_or((
            mask.origin_x + (mask.width as f32 * mask.px_mm) / 2.0,
            mask.origin_y + (mask.height as f32 * mask.px_mm) / 2.0,
            region.min_z,
        ));

        Self {
            id: format!("o{index}"),
            region_index: index,
            contact: Vec3::new(contact_x, contact_y, contact_z),
            base_z: contact_z,
            area_mm2: region.projected_area_mm2,
            overhang_angle_deg: region.angle_deg,
            surface_area_mm2: region.area_mm2,
            max_z: region.max_z,
            steep_flat: region.steep_flat,
            triangle_ids: region.triangle_ids.clone(),
            surface_normal: region.normal,
            footprint: mask.clone(),
            perimeter_loops: region.perimeter_loops.clone(),
            interior: false,
        }
    }
}

/// Which detector produced a candidate (`CandidatePoint['source']`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CandidateSource {
    Overhang,
}

/// `CandidatePoint`, with the geometry in doubles and the sizing fields this
/// stage does not fill.
#[derive(Clone, Debug)]
pub(crate) struct Candidate {
    pub id: String,
    pub tip_pos: DVec3,
    pub tip_normal: DVec3,
    pub source: CandidateSource,
    pub island_area_mm2: f64,
    pub z_height: f64,
    pub priority: f64,
    /// Density-grid point: a standalone contact of the region lattice or ring.
    pub grid_point: bool,
    /// Index of the island (region) this candidate came from.
    pub region_index: usize,
}

impl Candidate {
    fn from_island(island: &Island) -> Self {
        Self {
            id: island.id.clone(),
            tip_pos: island.contact.as_dvec3(),
            // Placeholder — `resolveSurfaceNormal` fills it in at placement
            // time; the grid path uses the classifier's per-face normal.
            tip_normal: DVec3::new(0.0, 0.0, -1.0),
            source: CandidateSource::Overhang,
            island_area_mm2: island.area_mm2 as f64,
            z_height: island.base_z as f64,
            priority: 0.0,
            grid_point: false,
            region_index: island.region_index,
        }
    }
}

/// `steepFlatNeedsCoverage`: should this steep flat be covered, given the
/// pose's verdict? Size, not share, is what makes a flat an anchoring surface,
/// so a flat at least [`STEEP_FLAT_ANCHOR_MIN_AREA_MM2`] keeps its contacts
/// even when the pose needs no rescuing.
pub(crate) fn steep_flat_needs_coverage(
    surface_area_mm2: f32,
    verdict_needs_coverage: bool,
) -> bool {
    if verdict_needs_coverage {
        return true;
    }
    surface_area_mm2 >= STEEP_FLAT_ANCHOR_MIN_AREA_MM2
}

/// `influenceRadiusMm`: a tip's coverage disc widens the higher above it you
/// go, piecewise-linear through [`INFLUENCE_RADIUS_KNOTS`], capped at
/// [`MAX_INFLUENCE_RADIUS_MM`].
pub(crate) fn influence_radius_mm(diff_z_mm: f64) -> f64 {
    let z = diff_z_mm.max(0.0);
    let knots = INFLUENCE_RADIUS_KNOTS;
    if z <= knots[0].0 {
        return knots[0].1;
    }
    for i in 1..knots.len() {
        if z <= knots[i].0 {
            let (z0, r0) = knots[i - 1];
            let (z1, r1) = knots[i];
            return r0 + ((r1 - r0) * (z - z0)) / (z1 - z0);
        }
    }
    knots[knots.len() - 1].1
}

/// `generateCandidates`: drop islands below the area floor, emit one or two
/// candidates each, score them and sort by priority (stable, descending).
pub(crate) fn generate_candidates(islands: &[Island], settings: &SupportSettings) -> Vec<Candidate> {
    if islands.is_empty() {
        return Vec::new();
    }
    let eligible: Vec<&Island> = islands
        .iter()
        .filter(|island| island.area_mm2 >= settings.min_island_area_mm2)
        .collect();

    let mut candidates: Vec<Candidate> = eligible
        .iter()
        .flat_map(|island| candidates_from_island(island))
        .collect();
    if candidates.is_empty() {
        return candidates;
    }

    let max_z = candidates
        .iter()
        .map(|c| c.z_height)
        .fold(1.0f64, f64::max);
    let max_area = candidates
        .iter()
        .map(|c| c.island_area_mm2)
        .fold(0.01f64, f64::max);
    for candidate in &mut candidates {
        candidate.priority = compute_priority(candidate, max_z, max_area);
    }
    candidates.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    candidates
}

/// `candidatesFromIsland`: one island yields one or two candidates by span.
/// Sub-head specks get a single tip at the bbox centre; narrow islands in the
/// two-point band split into a symmetric pair, each carrying half the area.
fn candidates_from_island(island: &Island) -> Vec<Candidate> {
    let base = Candidate::from_island(island);
    let points = island.footprint.points();
    if points.is_empty() {
        return vec![base];
    }

    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for [x, y, _] in &points {
        min_x = min_x.min(*x);
        max_x = max_x.max(*x);
        min_y = min_y.min(*y);
        max_y = max_y.max(*y);
    }
    let ext_x = max_x - min_x;
    let ext_y = max_y - min_y;
    let major = ext_x.max(ext_y);

    if major <= ISLAND_SUB_HEAD_MM {
        return vec![Candidate {
            tip_pos: DVec3::new(
                ((min_x + max_x) / 2.0) as f64,
                ((min_y + max_y) / 2.0) as f64,
                island.contact.z as f64,
            ),
            ..base
        }];
    }

    let minor = ext_x.min(ext_y);
    if major >= ISLAND_TWO_POINT_MIN_MM
        && major < ISLAND_TWO_POINT_MAX_MM
        && minor < ISLAND_TWO_POINT_MAX_WIDTH_MM
    {
        let along_x = ext_x >= ext_y;
        let lo = if along_x { min_x } else { min_y };
        let span = if along_x { ext_x } else { ext_y };
        let fixed = if along_x {
            (min_y + max_y) / 2.0
        } else {
            (min_x + max_x) / 2.0
        };
        let half_area = base.island_area_mm2 / 2.0;
        return [0.25f32, 0.75].iter().enumerate().map(|(i, t)| {
            let along = lo + span * t;
            let suffix = if i == 0 { "a" } else { "b" };
            let (x, y) = if along_x {
                (along, fixed)
            } else {
                (fixed, along)
            };
            Candidate {
                id: format!("{}-{suffix}", island.id),
                tip_pos: DVec3::new(x as f64, y as f64, island.contact.z as f64),
                island_area_mm2: half_area,
                ..Candidate::from_island(island)
            }
        }).collect();
    }

    vec![base]
}

/// `computePriority`: 60% area, 30% Z-height (lower = more urgent), plus a
/// source bonus this stage's single source never earns.
fn compute_priority(c: &Candidate, max_z: f64, max_area: f64) -> f64 {
    let area_score = (c.island_area_mm2 / max_area.max(0.01)) * 0.6;
    let z_score = (1.0 - c.z_height / max_z.max(1.0)) * 0.3;
    let source_score = 0.0;
    area_score + z_score + source_score
}

/// The integer key of a 2D cell — `cellKey` in `voxelFootprint.ts`.
fn cell_key(cx: i64, cy: i64) -> i64 {
    (cx + 32768) * 65536 + (cy + 32768)
}

/// JS `Math.round`: half-way values round toward +∞.
fn js_round(value: f64) -> f64 {
    (value + 0.5).floor()
}

/// `deduplicateCandidates`: overhang lattice pairs suppress on a 2D (XY) disc
/// that widens with vertical separation along the influence curve, except pairs
/// farther apart in Z than [`SUPPORT_RESTSTACK_DELTA_MM`].
pub(crate) fn deduplicate_candidates(
    candidates: Vec<Candidate>,
    settings: &SupportSettings,
) -> Vec<Candidate> {
    if candidates.len() <= 1 {
        return candidates;
    }
    let base_radius = settings.tip_influence_radius_mm as f64;
    if base_radius <= 0.0 {
        let mut sorted = candidates;
        sorted.sort_by(|a, b| b.priority.total_cmp(&a.priority));
        return sorted;
    }

    // The suppression disc widens with vertical separation; the neighbour
    // search spans the max grown radius.
    let max_gate = base_radius + (MAX_INFLUENCE_RADIUS_MM - TIP_COVERAGE_RADIUS_MM);
    let cell_size = base_radius;
    let cell_range = (max_gate / cell_size).ceil().max(1.0) as i64;

    // Bucket the *retained* candidates by XY cell as they are kept: only a
    // retained candidate can suppress a later one.
    let mut grid: std::collections::HashMap<i64, Vec<usize>> = std::collections::HashMap::new();

    let mut sorted = candidates;
    sorted.sort_by(|a, b| b.priority.total_cmp(&a.priority));

    let mut retained: Vec<Candidate> = Vec::with_capacity(sorted.len());
    for candidate in sorted {
        let cx = js_round(candidate.tip_pos.x / cell_size);
        let cy = js_round(candidate.tip_pos.y / cell_size);

        let mut duplicate = false;
        for dx in -cell_range..=cell_range {
            if duplicate {
                break;
            }
            for dy in -cell_range..=cell_range {
                let Some(bucket) = grid.get(&cell_key(cx as i64 + dx, cy as i64 + dy)) else {
                    continue;
                };
                for &r in bucket {
                    let r = &retained[r];
                    let ddx = candidate.tip_pos.x - r.tip_pos.x;
                    let ddy = candidate.tip_pos.y - r.tip_pos.y;
                    let dz = (candidate.tip_pos.z - r.tip_pos.z).abs();
                    // Discrete islands are must-support points: a flat 3D ball
                    // only. The grown disc applies to overhang pairs, where
                    // merging is the goal.
                    if candidate.source != CandidateSource::Overhang
                        || r.source != CandidateSource::Overhang
                    {
                        let ddz = candidate.tip_pos.z - r.tip_pos.z;
                        if ddx * ddx + ddy * ddy + ddz * ddz <= base_radius * base_radius {
                            duplicate = true;
                            break;
                        }
                        continue;
                    }
                    // Restack: vertically separated shelves never suppress each
                    // other, however close in XY.
                    if dz > SUPPORT_RESTSTACK_DELTA_MM {
                        continue;
                    }
                    let gate =
                        base_radius + (influence_radius_mm(dz) - TIP_COVERAGE_RADIUS_MM).max(0.0);
                    if ddx * ddx + ddy * ddy <= gate * gate {
                        duplicate = true;
                        break;
                    }
                }
                if duplicate {
                    break;
                }
            }
        }

        if duplicate {
            continue;
        }
        let index = retained.len();
        retained.push(candidate);
        grid.entry(cell_key(cx as i64, cy as i64))
            .or_default()
            .push(index);
    }

    retained.sort_by(|a, b| b.priority.total_cmp(&a.priority));
    retained
}
