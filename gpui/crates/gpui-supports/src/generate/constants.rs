//! The auto-support placement constants this stage reads, each named after the
//! source declaration it came from.
//!
//! Sources, in the order the port walks them:
//!
//! - `src/volumeAnalysis/Islands/useIslands.ts` — the scan settings
//!   (`OVERHANG_SELF_SUPPORT_ANGLE_DEG`, `OVERHANG_FOOTPRINT_PX_MM`).
//! - `src-tauri/src/overhang.rs` — the classifier's own thresholds
//!   (`STEEP_FLAT_MAX_ANGLE_DEG`, `STEEP_FLAT_MIN_AREA_MM2`,
//!   `STEEP_FLAT_NORMAL_TOL_DEG`, the 0.25 mm perimeter inset).
//! - `src/supports/autoSupport/constants.ts` — the shared placement constants.
//! - `src/supports/autoSupport/gridPlacement.ts` — the density-grid constants.
//! - `src/supports/autoSupport/coverage.ts` and `poseStability.ts` — the two
//!   constants the candidate filters read.

/// `OVERHANG_SELF_SUPPORT_ANGLE_DEG` — the scan's default self-support angle.
/// `src/volumeAnalysis/Islands/useIslands.ts`; the settings' own
/// `overhangSelfSupportAngleDeg` (`constants.ts`, `OVERHANG_SELF_SUPPORT_ANGLE_DEG`)
/// is what a run passes.
pub(crate) const OVERHANG_SELF_SUPPORT_ANGLE_DEG: f32 = 45.0;

/// `OVERHANG_FOOTPRINT_PX_MM` — resolution of the projected-footprint masks the
/// classifier emits (`useIslands.ts`). Everything downstream quantizes on this
/// grid: the mask, the contact voxels, the erosion and the boundary fallback.
pub(crate) const OVERHANG_FOOTPRINT_PX_MM: f32 = 0.25;

/// `STEEP_FLAT_MAX_ANGLE_DEG` — steepest face a steep-flat patch may have.
/// `src-tauri/src/overhang.rs`.
pub(crate) const STEEP_FLAT_MAX_ANGLE_DEG: f32 = 85.0;

/// `STEEP_FLAT_MIN_AREA_MM2` — 3D area a steep planar patch must reach before
/// it counts as a topple lever. `src-tauri/src/overhang.rs`.
pub(crate) const STEEP_FLAT_MIN_AREA_MM2: f32 = 150.0;

/// `STEEP_FLAT_NORMAL_TOL_DEG` — how far a triangle's normal may sit from the
/// growing patch's running mean. `src-tauri/src/overhang.rs`.
pub(crate) const STEEP_FLAT_NORMAL_TOL_DEG: f32 = 20.0;

/// The inset `build_perimeter_loops` applies (`inset_mm = 0.25` in
/// `src-tauri/src/overhang.rs`, which is `PERIMETER_CONTACT_INSET_MM` from
/// `gridPlacement.ts`).
pub(crate) const PERIMETER_CONTACT_INSET_MM: f32 = 0.25;

/// `ANCHOR_HEIGHT_THRESHOLD_MM` — near-plate tips below this Z become the
/// near-plate type (`constants.ts`; read by the `tipHeight` placement rules on
/// `trunk` and `stump` in `supportTypeRegistry.ts`).
pub(crate) const ANCHOR_HEIGHT_THRESHOLD_MM: f32 = 5.0;

/// `ISLAND_SUB_HEAD_MM` — bbox extent at/below which one tip at the bbox centre
/// serves the island (`constants.ts`).
pub(crate) const ISLAND_SUB_HEAD_MM: f32 = 0.5;

/// `ISLAND_TWO_POINT_MIN_MM` — shortest island that splits into a tip pair.
/// `constants.ts`.
pub(crate) const ISLAND_TWO_POINT_MIN_MM: f32 = 1.5;

/// `ISLAND_TWO_POINT_MAX_MM` — longest island that still splits into a pair;
/// also the length past which a sub-threshold region takes the density grid on
/// shape (`constants.ts`, read by `shouldUseDensityGrid`).
pub(crate) const ISLAND_TWO_POINT_MAX_MM: f32 = 6.0;

/// `ISLAND_TWO_POINT_MAX_WIDTH_MM` — the pair split needs a minor axis below
/// this (`constants.ts`).
pub(crate) const ISLAND_TWO_POINT_MAX_WIDTH_MM: f32 = 2.5;

/// `SUPPORT_RESTSTACK_DELTA_MM` — vertical restack allowance for the candidate
/// dedupe: candidates farther apart in Z never dedup each other (`constants.ts`).
pub(crate) const SUPPORT_RESTSTACK_DELTA_MM: f64 = 5.0;

/// `TIP_COVERAGE_RADIUS_MM` — a tip's coverage disc at its own height
/// (`coverage.ts`). The dedupe grows that disc along the influence curve.
pub(crate) const TIP_COVERAGE_RADIUS_MM: f64 = 3.0;

/// The influence curve's cap at unbounded height — the `6.0` the dedupe's
/// `maxGate` expression subtracts `TIP_COVERAGE_RADIUS_MM` from
/// (`candidateGeneration.ts`, `const maxGate = baseRadius + (6.0 -
/// TIP_COVERAGE_RADIUS_MM)`), and the last knot of `influenceRadiusMm`
/// (`constants.ts`).
pub(crate) const MAX_INFLUENCE_RADIUS_MM: f64 = 6.0;

/// `INFLUENCE_RADIUS_KNOTS` — piecewise-linear support influence growth
/// (`constants.ts`): `(0, 3.0) → (3.9, 4.0) → (15, 5.0) → (40, 6.0)`.
pub(crate) const INFLUENCE_RADIUS_KNOTS: [(f64, f64); 4] = [
    (0.0, 3.0),
    (3.9, 4.0),
    (15.0, 5.0),
    (40.0, 6.0),
];

/// `SLOPE_RELAX_RAMP_START` — where the relaxation toward the slope spacing
/// starts, as a fraction of the self-support angle (`constants.ts`).
pub(crate) const SLOPE_RELAX_RAMP_START: f64 = 0.6;

/// `GRID_SPACING_MIN_FACTOR` — spacing multiplier on flat ceilings
/// (`constants.ts`); the `flatDensityBoost` fallback.
pub(crate) const GRID_SPACING_MIN_FACTOR: f64 = 0.7;

/// `GRID_SPACING_MAX_FACTOR` — spacing multiplier at the self-support angle
/// (`constants.ts`); the `slopeRelaxFactor` fallback.
pub(crate) const GRID_SPACING_MAX_FACTOR: f64 = 1.3;

/// `INTERIOR_CAVITY_SPACING_SCALE` — interior ceilings are bridged, so their
/// grid is this much sparser (`constants.ts`, read by `gridPlacement.ts`).
pub(crate) const INTERIOR_CAVITY_SPACING_SCALE: f64 = 3.0;

/// `GRID_SPACING_FLOOR_MM` — absolute floor on grid spacing (`gridPlacement.ts`).
pub(crate) const GRID_SPACING_FLOOR_MM: f64 = 1.2;

/// `PERIMETER_RING_FACTOR` — boundary-ring spacing relative to the infill
/// spacing (`gridPlacement.ts`).
pub(crate) const PERIMETER_RING_FACTOR: f64 = 0.7;

/// `PERIMETER_SPACING_FLOOR_MM` — floor on the boundary-ring spacing
/// (`gridPlacement.ts`).
pub(crate) const PERIMETER_SPACING_FLOOR_MM: f64 = 1.0;

/// `MAX_GRID_CANDIDATES_PER_REGION` — per-region candidate cap
/// (`gridPlacement.ts`).
pub(crate) const MAX_GRID_CANDIDATES_PER_REGION: usize = 800;

/// `STEEP_FLAT_SPACING_MULTIPLIER` — a steep flat's sparse field, relative to
/// the formation density (`gridPlacement.ts`).
pub(crate) const STEEP_FLAT_SPACING_MULTIPLIER: f64 = 2.5;

/// `STEEP_FLAT_ANCHOR_BAND_FRACTION` — the lowest fraction of a steep flat's
/// height its anchoring contacts may occupy (`gridPlacement.ts`).
pub(crate) const STEEP_FLAT_ANCHOR_BAND_FRACTION: f64 = 0.35;

/// `STEEP_FLAT_ANCHOR_BAND_MM` — the floor on that band (`gridPlacement.ts`).
pub(crate) const STEEP_FLAT_ANCHOR_BAND_MM: f64 = 6.0;

/// `FOOTPRINT_TOLERANCE_MM` — the voxel surface sampler's nearest-voxel reach
/// (`gridPlacement.ts`).
pub(crate) const FOOTPRINT_TOLERANCE_MM: f64 = 0.25;

/// `STEEP_FLAT_ANCHOR_MIN_AREA_MM2` — a flat at least this big is worth
/// anchoring a part with whatever the pose verdict (`poseStability.ts`, read by
/// `steepFlatNeedsCoverage`).
pub(crate) const STEEP_FLAT_ANCHOR_MIN_AREA_MM2: f32 = 150.0;
